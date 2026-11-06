"""Offline validation of the two ordered vendor reset actions. Never accesses a device."""
import argparse
import hashlib
import json
import pathlib
import subprocess

from physical_capture import FIELDS
from usb_capture import match_completions, payload

ROOT = pathlib.Path(__file__).resolve().parent


def command(op, count=0, variant=0):
    data = bytearray(64)
    data[:3] = bytes((4, op, variant))
    data[8] = count
    return data.hex()


def map_clear(op):
    body = bytes(574) + b'\xaa\x55'
    return [(command(0x18), True), (command(op, 9), True)] + [
        (body[i:i+64].hex(), False) for i in range(0, 576, 64)
    ] + [(command(2), True), (command(0xf0), True)]


def reset_actions():
    keyboard = map_clear(0x11) + map_clear(0x27)
    clear = bytes(510) + b'\xaa\x55'
    factory = [(command(0x19), True), (command(0x15, 8), True)] + [
        (clear[i:i+64].hex(), False) for i in range(0, 512, 64)
    ] + [(command(2), True)] + keyboard
    lighting = bytearray(64)
    lighting[0:4] = bytes((11, 255, 0, 0))
    lighting[8:11] = bytes((1, 5, 3))
    lighting[14:16] = b'\xaa\x55'
    settings = bytearray(64)
    settings[1] = 1
    settings[6] = settings[8] = 2
    settings[62:] = b'\xaa\x55'
    factory += [(command(0x18), True), (command(0x13, 1), True),
                (lighting.hex(), False), (command(2), True), (command(0xf0), False),
                (command(0x18), True), (command(0x17, 1, 1), True),
                (settings.hex(), False), (command(2), True)]
    return [('reset_keyboard', keyboard), ('factory_reset', factory)]


def review_capture(rows, sha256):
    completed = match_completions(rows)
    writes = [r for r in rows if r['usb.urb_type'] == "'S'"
              and r.get('usb.bmRequestType') == '0x21'
              and r.get('usbhid.setup.bRequest') == '0x09'
              and r.get('usbhid.setup.wIndex') == '3'
              and r.get('usbhid.setup.wValue') == '0x0300']
    reads = [r for r in rows if r['usb.urb_type'] == "'S'"
             and r.get('usb.bmRequestType') == '0xa1'
             and r.get('usbhid.setup.bRequest') == '0x01'
             and r.get('usbhid.setup.wIndex') == '3'
             and r.get('usbhid.setup.wValue') == '0x0300']
    plans = reset_actions()
    assert len(writes) == sum(len(plan) for _, plan in plans), 'Unexpected reset write count'
    transactions, reports, used_reads = [], [], set()
    offset = 0
    for action, plan in plans:
        tx, replies = [], []
        for index, (expected, needs_reply) in enumerate(plan):
            r = writes[offset + index]
            assert payload(r) == expected, 'Unexpected reset bytes or action order'
            assert (r['usb.bus_id'], r['usb.device_address']) == (writes[0]['usb.bus_id'], writes[0]['usb.device_address'])
            completion = completed.get(r['frame.number'], {})
            assert completion.get('usb.urb_status') == '0', 'Failed or missing reset write'
            next_write = offset + index + 1
            end = int(writes[next_write]['frame.number']) if next_write < len(writes) else float('inf')
            assert int(r['frame.number']) < int(completion['frame.number']) < end
            report = dict(frame=int(r['frame.number']), time=float(r['frame.time_relative']),
                          bus=r['usb.bus_id'], device=r['usb.device_address'], payload=payload(r),
                          completion=int(completion['frame.number']), status='0', read_reply=needs_reply)
            tx.append(report)
            reports.append(report)
            matches = [read for read in reads if int(r['frame.number']) < int(read['frame.number']) < end
                       and (read['usb.bus_id'], read['usb.device_address']) == (r['usb.bus_id'], r['usb.device_address'])]
            assert len(matches) == int(needs_reply), 'Missing or unexpected reset reply read'
            if needs_reply:
                read = matches[0]
                reply = completed.get(read['frame.number'], {})
                data = bytes.fromhex(payload(reply))
                assert reply.get('usb.urb_status') == '0', 'Failed or missing reset reply'
                assert int(read['frame.number']) < int(reply['frame.number']) < end
                assert len(data) == 64 and data[:3] == bytes.fromhex(expected)[:3] and data[3] == 1, 'Rejected or mismatched reset reply'
                replies.append(dict(frame=int(reply['frame.number']), payload=data.hex()))
                used_reads.add(read['frame.number'])
        transactions.append(dict(action=action, writes=tx, replies=replies))
        offset += len(plan)
    assert len(used_reads) == len(reads), 'Unmatched reset read'
    review = dict(sha256=sha256, transaction_count=2, set_report_count=len(writes),
                  successful_completions=len(reports), verified_command_replies=len(reads),
                  action_label_basis='Capture filename and stated action order; operator confirmation recorded separately.',
                  reset_keyboard='Clear Top and Fn override tables.',
                  factory_reset='0419; clear eight-block 0415 table; clear Top/Fn; restore captured lighting and settings.',
                  unknown_clear_table='Purpose of the 0415 storage area and 0419 command is not independently established.',
                  factory_settings=dict(level=2, sleep='5min', disable_win=False, disable_alt_f4=False, disable_alt_tab=False),
                  factory_lighting=dict(mode_id=11, rgb=[255,0,0], colorful=True, brightness=5, speed=3),
                  factory_lighting_refresh_read=False, physical_app_verification=False)
    return review, dict(sha256=sha256, transactions=transactions), reports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path)
    parser.add_argument('--output-dir', type=pathlib.Path, default=ROOT.parent.parent / 'capture-output')
    args = parser.parse_args()
    command_line = ['tshark','-r',str(args.capture),'-T','fields']
    for field in FIELDS:
        command_line += ['-e',field]
    rows = [dict(zip(FIELDS,line.split('\t'))) for line in subprocess.check_output(command_line,text=True).splitlines()]
    review, fixtures, reports = review_capture(rows,hashlib.sha256(args.capture.read_bytes()).hexdigest())
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for suffix, value in [('review',review),('fixtures',fixtures),('set-reports',reports)]:
        (args.output_dir / f'reset-keyboard-and-factory-reset-{suffix}.json').write_text(json.dumps(value,indent=2)+'\n')
    print(json.dumps(review,indent=2))


if __name__ == '__main__':
    main()
