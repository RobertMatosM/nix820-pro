"""Read-only review of the operator-labelled System Settings sweep.

Run from app/: python3 analysis/settings_capture.py [capture.pcapng]
Unrelated input traffic is discarded. No device access.
"""
import hashlib
import json
import pathlib
import subprocess
import sys

from physical_capture import FIELDS
from usb_capture import match_completions, payload

ROOT = pathlib.Path(__file__).parent
# Labels come from the user's stated order, not packet contents alone.
STATES = [(level, 1, False, False, False) for level in range(1, 6)] + [
    (5, sleep, False, False, False) for sleep in (0, 1, 2, 3)
] + [(5, 3, True, False, False), (5, 3, True, True, False),
     (5, 3, True, True, True)]


def review_capture(rows, sha256):
    completions = match_completions(rows)
    writes = [r for r in rows if r['usb.urb_type'] == "'S'"
              and r['usb.bmRequestType'] == '0x21'
              and r['usbhid.setup.bRequest'] == '0x09'
              and r['usbhid.setup.wIndex'] == '3'
              and r['usbhid.setup.wValue'] == '0x0300']
    reads = [r for r in rows if r['usb.urb_type'] == "'S'"
             and r['usb.bmRequestType'] == '0xa1'
             and r['usbhid.setup.bRequest'] == '0x01'
             and r['usbhid.setup.wIndex'] == '3'
             and r['usbhid.setup.wValue'] == '0x0300']
    assert len(writes) == len(STATES) * 4, 'Incomplete or unexpected settings sweep'
    transactions, reports = [], []
    previous = None
    for n, (level, sleep, win, f4, tab) in enumerate(STATES):
        tx = writes[n * 4:n * 4 + 4]
        expected_data = bytearray(64)
        expected_data[1] = 1
        expected_data[2:5] = bytes((tab, f4, win))
        expected_data[6] = sleep
        expected_data[8] = level
        expected_data[62:] = b'\xaa\x55'
        commands = []
        for op in (0x18, 0x17, 0x02):
            command = bytearray(64)
            command[:2] = bytes((4, op))
            if op == 0x17:
                command[2] = command[8] = 1
            commands.append(command)
        expected = [commands[0], commands[1], expected_data, commands[2]]
        assert all((r['usb.bus_id'], r['usb.device_address']) ==
                   (tx[0]['usb.bus_id'], tx[0]['usb.device_address']) for r in tx)
        for r, data in zip(tx, expected):
            assert bytes.fromhex(payload(r)) == data, 'Unexpected settings bytes or action order'
            completion = completions.get(r['frame.number'], {})
            assert completion.get('usb.urb_status') == '0', 'Failed or missing USB write'
            reports.append(dict(frame=int(r['frame.number']), time=float(r['frame.time_relative']),
                                bus=r['usb.bus_id'], device=r['usb.device_address'],
                                payload=payload(r), completion=int(completion['frame.number']),
                                status=completion['usb.urb_status']))
        replies = []
        for index in (0, 1, 3):
            command = tx[index]
            next_index = n * 4 + index + 1
            end = int(writes[next_index]['frame.number']) if next_index < len(writes) else float('inf')
            matches = [r for r in reads if r['usb.bus_id'] == command['usb.bus_id']
                       and r['usb.device_address'] == command['usb.device_address']
                       and int(command['frame.number']) < int(r['frame.number']) < end]
            assert len(matches) == 1, 'Missing or ambiguous command read'
            reply = completions.get(matches[0]['frame.number'], {})
            data = bytes.fromhex(payload(reply))
            assert reply.get('usb.urb_status') == '0', 'Failed or missing command read'
            assert int(matches[0]['frame.number']) < int(reply['frame.number']) < end
            assert len(data) == 64 and data[:3] == expected[index][:3] and data[3] == 1, 'Command rejected'
            replies.append(dict(frame=int(reply['frame.number']), payload=data.hex()))
        transactions.append(dict(frame=int(tx[2]['frame.number']),
                                 time=float(tx[2]['frame.time_relative']),
                                 level=level, sleep=sleep, disable_win=win,
                                 disable_alt_f4=f4, disable_alt_tab=tab,
                                 writes=[payload(r) for r in tx], replies=replies,
                                 changed_offsets=None if previous is None else [
                                     i for i in range(64) if expected_data[i] != previous[i]]))
        previous = expected_data
    return dict(sha256=sha256, transaction_count=len(transactions),
                set_report_count=len(writes), successful_completions=len(reports),
                verified_command_replies=sum(len(t['replies']) for t in transactions),
                unique_settings=len({t['writes'][2] for t in transactions}),
                field_offsets=dict(response_level=8, sleep=6, disable_win=4,
                                   disable_alt_f4=3, disable_alt_tab=2),
                sleep_choices=['No sleep', '1 min', '5 min', '30 min'],
                interpretation='Field meanings derived from operator-labelled action order; physical effects and persistence unverified.',
                reset_commands_captured=False), transactions, reports


def main():
    if len(sys.argv) != 2:
        raise SystemExit('Usage: python3 '+pathlib.Path(__file__).name+' PRIVATE_CAPTURE.pcapng')
    path = pathlib.Path(sys.argv[1])
    args = ['tshark', '-r', str(path), '-T', 'fields']
    for field in FIELDS:
        args += ['-e', field]
    rows = [dict(zip(FIELDS, line.split('\t'))) for line in subprocess.check_output(args, text=True).splitlines()]
    review, transactions, reports = review_capture(rows, hashlib.sha256(path.read_bytes()).hexdigest())
    (ROOT.parent.parent / 'capture-output').mkdir(exist_ok=True)
    for name, data in [('review', review), ('fixtures', dict(sha256=review['sha256'], transactions=transactions)),
                       ('set-reports', reports)]:
        (ROOT.parent.parent / 'capture-output' / f'system-settings-{name}.json').write_text(json.dumps(data, indent=2) + '\n')
    print(json.dumps(review, indent=2))


if __name__ == '__main__':
    main()
