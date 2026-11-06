"""Offline extraction of the operator-labelled Fn+I -> Insert capture."""
import argparse
import hashlib
import json
import pathlib
import subprocess

from physical_capture import FIELDS, review_capture
from usb_capture import payload

ROOT = pathlib.Path(__file__).resolve().parent
RESERVED_KEYS = ['Esc', 'Backspace', 'Home', 'Win', 'Fn', 'Backslash/Pipe']


def review_fn_capture(rows, sha256):
    review, tables, reports = review_capture(rows, sha256, map_command='0427')
    assert len(tables) == 2, 'Expected the captured remap and all-zero Fn tables'
    by_frame = {int(row['frame.number']): row for row in rows}
    transactions = []
    for index, table in enumerate(tables):
        expected = bytearray(576)
        if index == 0:
            expected[180:184] = bytes.fromhex('02004900')
        expected[574:] = b'\xaa\x55'
        assert bytes.fromhex(table['body']) == expected, 'Unexpected Fn source or destination data'
        replies = [by_frame[frame] for frame in table['command_reply_frames']]
        transactions.append(dict(action='i_to_insert' if index == 0 else 'clear_fn_overrides_observed',
            **table, writes=reports[index*13:(index+1)*13],
            replies=[dict(frame=int(row['frame.number']), bus=row['usb.bus_id'],
                          device=row['usb.device_address'], status=row['usb.urb_status'],
                          payload=payload(row)) for row in replies]))
    review.update(layer='Fn', layer_confirmed=True, physical_labels_confirmed=True,
                  source_key='I', source_slot=45, source_offset=180, destination='Insert',
                  destination_record='02004900', map_command='0427', top_map_command='0411',
                  reserved_keys=RESERVED_KEYS,
                  reserved_key_basis='Operator transcribed the original driver hint for Fn-layer source keys.',
                  label_basis='Operator explicitly assigned Insert to physical I on Fn layer.',
                  second_table='Observed all-zero overrides plus aa55; operator did not specify the clearing gesture.',
                  shared_source_layout='I independently matches Top slot 45; other Fn source names reuse the Top layout as an inference.',
                  physical_key_output_verified=False)
    return review, dict(sha256=sha256, layer='Fn', source_key='I', source_slot=45,
                        source_offset=180, transactions=transactions), reports


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
    review, fixtures, reports = review_fn_capture(rows, hashlib.sha256(args.capture.read_bytes()).hexdigest())
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for suffix, value in [('review', review), ('fixtures', fixtures), ('set-reports', reports)]:
        (args.output_dir / f'fn-layer-remap-test-{suffix}.json').write_text(json.dumps(value, indent=2) + '\n')
    print(f"{len(fixtures['transactions'])} Fn transactions; {review['successful_completions']} successful writes; {review['verified_command_replies']} positive replies")


if __name__ == '__main__':
    main()
