"""Offline review of the operator-labelled Esc media destination sweep.

Run from app/: python3 analysis/media_capture.py [capture.pcapng] [--output-dir DIR]
Only configuration transfers are exported. No device access or input-event export.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess

from physical_capture import FIELDS, review_capture
from usb_capture import payload

ROOT = pathlib.Path(__file__).resolve().parent
# Operator order plus the byte records observed in this specific capture.
ACTIONS = [
    ('my_computer', 'My Computer', '03940100'),
    ('calculator', 'Calculator', '03920100'),
    ('mail', 'Mail', '038a0100'),
    ('multimedia', 'Multimedia', '03830100'),
    ('play_pause', 'Play/Pause', '03cd0000'),
    ('stop', 'Stop', '03b70000'),
    ('previous_track', 'Previous Track', '03b60000'),
    ('next_track', 'Next Track', '03b50000'),
    ('volume_up', 'Volume +', '03e90000'),
    ('volume_down', 'Volume', '03ea0000'),
    ('mute', 'Mute', '03e20000'),
    ('browser_home', 'Homepage', '03230200'),
    ('browser_refresh', 'Web Page - Refresh', '03270200'),
    ('browser_stop', 'Web Page - Stop', '03260200'),
    ('browser_back', 'Web Page - Back', '03240200'),
    ('browser_forward', 'Web Page - Forward', '03250200'),
    ('browser_favorites', 'Web Page - Favorites', '032a0200'),
    ('browser_search', 'Web Page - Search', '03210200'),
]


def review_media_capture(rows, sha256):
    review, tables, reports = review_capture(rows, sha256)
    assert len(tables) == len(ACTIONS), 'Expected the 18 operator-labelled actions'
    by_frame = {int(row['frame.number']): row for row in rows}
    transactions = []
    for index, (table, (action, label, record)) in enumerate(zip(tables, ACTIONS)):
        expected = bytearray(576)
        expected[4:8] = bytes.fromhex(record)  # Confirmed physical Esc source slot 1.
        expected[574:] = b'\xaa\x55'
        assert bytes.fromhex(table['body']) == expected, 'Unexpected source, action order or other map bytes'
        replies = [by_frame[frame] for frame in table['command_reply_frames']]
        transactions.append(dict(action=action, operator_label=label, record=record, **table,
            writes=reports[index*13:(index+1)*13],
            replies=[dict(frame=int(row['frame.number']), bus=row['usb.bus_id'],
                          device=row['usb.device_address'], status=row['usb.urb_status'],
                          payload=payload(row)) for row in replies]))
    review.update(source_key='Esc', source_slot=1, source_offset=4,
                  physical_labels_confirmed=True, layer='Top', layer_confirmed=True,
                  layer_basis='Continuing the Top-layer remapping investigation; 0411 matches the earlier operator-labelled Top capture.',
                  label_basis='Operator supplied the ordered 18 media actions; only Esc changes in the capture.',
                  media_destination_count=len(ACTIONS),
                  volume_label_note='Operator wrote Volume at action 10; captured consumer usage 00ea is Volume Decrement, displayed as Volume Down.',
                  actions=[dict(action=tx['action'], operator_label=tx['operator_label'],
                                record=tx['record'], map_frame=tx['frame'], time=tx['time'])
                           for tx in transactions],
                  physical_action_output_verified=False, default_restore_captured=False)
    return review, dict(sha256=sha256, source_key='Esc', source_slot=1,
                        source_offset=4, layer='Top', transactions=transactions), reports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path)
    parser.add_argument('--output-dir', type=pathlib.Path, default=ROOT.parent.parent / 'capture-output')
    args = parser.parse_args()
    command = ['tshark', '-r', str(args.capture), '-T', 'fields']
    for field in FIELDS:
        command += ['-e', field]
    rows = [dict(zip(FIELDS, line.split('\t'))) for line in
            subprocess.check_output(command, text=True).splitlines()]
    review, fixtures, reports = review_media_capture(rows, hashlib.sha256(args.capture.read_bytes()).hexdigest())
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for suffix, value in [('review', review), ('fixtures', fixtures), ('set-reports', reports)]:
        (args.output_dir / f'media-capture-remap-test-{suffix}.json').write_text(json.dumps(value, indent=2) + '\n')
    print(f"{len(fixtures['transactions'])} media actions; {review['successful_completions']} successful writes; {review['verified_command_replies']} positive replies")


if __name__ == '__main__':
    main()
