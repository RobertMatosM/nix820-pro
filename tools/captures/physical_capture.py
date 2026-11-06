"""Review source-position sweeps offline; never opens a keyboard.

Run from app/: python3 analysis/physical_capture.py [capture.pcapng]
Only interface-3 feature reports are saved; unrelated input traffic is discarded.
"""
import collections
import hashlib
import json
import pathlib
import subprocess
import sys

from usb_capture import match_completions, payload

ROOT = pathlib.Path(__file__).parent
FIELDS = ['frame.number', 'frame.time_relative', 'usb.bus_id', 'usb.device_address',
          'usb.endpoint_address', 'usb.urb_type', 'usb.urb_id', 'usb.urb_status',
          'usb.bmRequestType', 'usbhid.setup.bRequest', 'usbhid.setup.wIndex',
          'usbhid.setup.wValue', 'usbhid.data', 'usb.capdata']


def review_capture(all_rows, sha256, map_command='0411'):
    assert map_command in ('0411', '0427'), 'Unsupported map command'
    completions = match_completions(all_rows)
    writes = [r for r in all_rows if r['usb.urb_type'] == "'S'"
              and r['usb.bmRequestType'] == '0x21'
              and r['usbhid.setup.bRequest'] == '0x09'
              and r['usbhid.setup.wIndex'] == '3'
              and r['usbhid.setup.wValue'] == '0x0300'
              and len(payload(r)) == 128]
    reads = [r for r in all_rows if r['usb.urb_type'] == "'S'"
             and r['usb.bmRequestType'] == '0xa1'
             and r['usbhid.setup.bRequest'] == '0x01'
             and r['usbhid.setup.wIndex'] == '3'
             and r['usbhid.setup.wValue'] == '0x0300']
    assert writes and len(writes) % 13 == 0, 'Incomplete map transaction'
    tables, sources, bodies = [], [], []
    previous = None
    reports = []
    for r in writes:
        reply = completions.get(r['frame.number'], {})
        reports.append(dict(frame=int(r['frame.number']), time=float(r['frame.time_relative']),
                            bus=r['usb.bus_id'], device=r['usb.device_address'], interface=3,
                            payload=payload(r), completion=reply.get('frame.number'),
                            status=reply.get('usb.urb_status')))
    for i in range(0, len(writes), 13):
        tx = writes[i:i + 13]
        assert all((r['usb.bus_id'], r['usb.device_address']) ==
                   (tx[0]['usb.bus_id'], tx[0]['usb.device_address']) for r in tx)
        assert all(completions.get(r['frame.number'], {}).get('usb.urb_status') == '0'
                   for r in tx), 'Failed or missing USB completion'
        for index, prefix in ((0, '0418'), (1, map_command), (11, '0402'), (12, '04f0')):
            assert payload(tx[index]).startswith(prefix), 'Unexpected command order'
        assert bytes.fromhex(payload(tx[1]))[8] == 9
        reply_frames = []
        for index in (0, 1, 11, 12):
            command = tx[index]
            end = int(writes[i + index + 1]['frame.number']) if i + index + 1 < len(writes) else float('inf')
            matches = [r for r in reads if r['usb.bus_id'] == command['usb.bus_id']
                       and r['usb.device_address'] == command['usb.device_address']
                       and int(command['frame.number']) < int(r['frame.number']) < end]
            assert len(matches) == 1, 'Missing or ambiguous command read'
            reply = completions.get(matches[0]['frame.number'], {})
            data = bytes.fromhex(payload(reply))
            assert reply.get('usb.urb_status') == '0'
            assert int(command['frame.number']) < int(reply['frame.number']) < end
            assert len(data) == 64 and data[:3] == bytes.fromhex(payload(command))[:3]
            assert data[3] == 1, 'Command rejected'
            reply_frames.append(int(reply['frame.number']))
        body = b''.join(bytes.fromhex(payload(r)) for r in tx[2:11])
        assert len(body) == 576 and body[572:] == b'\x00\x00\xaa\x55'
        changed = None if previous is None else [j // 4 for j in range(0, 572, 4)
                                                  if body[j:j + 4] != previous[j:j + 4]]
        nonzero = [j // 4 for j in range(0, 572, 4) if any(body[j:j + 4])]
        # The first table has no captured predecessor: its nonzero records
        # are observed states, not proven changes from an empty keyboard.
        for slot in nonzero:
            if not any(s['slot'] == slot for s in sources):
                sources.append(dict(slot=slot, offset=slot * 4,
                                    first_frame=int(tx[1]['frame.number']),
                                    time=float(tx[1]['frame.time_relative']),
                                    record=body[slot * 4:slot * 4 + 4].hex()))
        tables.append(dict(frame=int(tx[1]['frame.number']),
                           time=float(tx[1]['frame.time_relative']), command=map_command,
                           body=body.hex(), data_frames=[int(r['frame.number']) for r in tx[2:11]],
                           command_reply_frames=reply_frames, changed_slots=changed,
                           nonzero_slots=nonzero, sha256=hashlib.sha256(body).hexdigest()))
        bodies.append(body)
        previous = body
    return dict(sha256=sha256, set_report_count=len(writes),
                successful_completions=sum(r['status'] == '0' for r in reports),
                table_count=len(tables), unique_tables=len(set(bodies)),
                verified_command_replies=4 * len(tables), source_slot_count=len(sources),
                repeated_updates=sum(t['changed_slots'] == [] for t in tables),
                commands=sorted({t['command'] for t in tables}),
                source_slots=sources,
                record_counts=dict(collections.Counter(s['record'] for s in sources)),
                physical_labels_confirmed=False, layer_confirmed=False), tables, reports


def main():
    if len(sys.argv) != 2:
        raise SystemExit('Usage: python3 '+pathlib.Path(__file__).name+' PRIVATE_CAPTURE.pcapng')
    path = pathlib.Path(sys.argv[1])
    args = ['tshark', '-r', str(path), '-T', 'fields']
    for field in FIELDS:
        args += ['-e', field]
    rows = [dict(zip(FIELDS, line.split('\t'))) for line in
            subprocess.check_output(args, text=True).splitlines()]
    review, tables, reports = review_capture(rows, hashlib.sha256(path.read_bytes()).hexdigest())
    stem = 'exclusive-config-physical-test'
    (ROOT.parent.parent / 'capture-output').mkdir(exist_ok=True)
    for suffix, data in [('review', review), ('fixtures', dict(sha256=review['sha256'], tables=tables)),
                         ('set-reports', reports)]:
        (ROOT.parent.parent / 'capture-output' / f'{stem}-{suffix}.json').write_text(json.dumps(data, indent=2) + '\n')
    print(json.dumps({k: v for k, v in review.items() if k != 'source_slots'}, indent=2))


if __name__ == '__main__':
    main()
