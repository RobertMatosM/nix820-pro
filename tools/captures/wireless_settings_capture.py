"""Review labelled wireless settings/reset captures without sending USB commands."""
import argparse
import csv
import hashlib
import io
import json
import pathlib
import subprocess

from usb_capture import match_completions, payload
from wireless_capture import FIELDS, packet

STATES = {
    'levels': [(level, 2, False, False, False) for level in range(1, 6)],
    'sleep': [(2, sleep, False, False, False) for sleep in (0, 0, 1, 2, 3)],
    'locks': [(2, 1, True, False, False), (2, 1, True, True, False),
              (2, 1, True, True, True), (2, 1, True, True, False),
              (2, 1, True, False, False), (2, 1, False, False, False)],
}
OPCODES = {'levels': 7, 'sleep': 7, 'locks': 7, 'reset-keyboard': 0x1f, 'factory-reset': 0x0f}


def expected_settings(state):
    level, sleep, win, f4, tab = state
    data = bytearray(32)
    data[:3] = bytes([7, 16, 0])
    data[4] = 1
    data[5:8] = bytes([tab, f4, win])
    data[9], data[11] = sleep, level
    data[17:19] = bytes.fromhex('aa55')
    data[31] = sum(data[:31]) % 256
    return data


def review_capture(rows, sha256, kind, bus, device):
    scoped = [r for r in rows if r['usb.bus_id'] == str(bus)
              and r['usb.device_address'] == str(device)]
    completed = match_completions(scoped)
    paired_reads = {r['frame.number'] for r in completed.values()}
    writes = [r for r in scoped if r['usb.urb_type'] == "'S'" and
              r['usb.transfer_type'] == '0x01' and r['usb.endpoint_address'] == '0x05']
    reads = [r for r in scoped if r['usb.urb_type'] == "'C'" and
             r['usb.transfer_type'] == '0x01' and r['usb.endpoint_address'] == '0x85' and payload(r)]
    # Validate every scoped output, including extra 20 01 00 requests.
    for write in writes:
        packet(write)
        completion = completed.get(write['frame.number'], {})
        assert completion.get('usb.urb_status') == '0', 'Failed or missing USB write completion'
        assert completion['usb.endpoint_address'] == '0x05'
        assert int(completion['frame.number']) > int(write['frame.number'])
    targets = [r for r in writes if bytes.fromhex(payload(r))[0] == OPCODES[kind]]
    assert len(targets) == (len(STATES[kind]) if kind in STATES else 1), 'Unexpected action count'
    transactions = []
    for i, write in enumerate(targets):
        data = packet(write)
        if kind in STATES:
            assert data == expected_settings(STATES[kind][i]), 'Unexpected settings state/order'
        else:
            expected = bytearray(32)
            expected[0] = expected[31] = OPCODES[kind]
            assert data == expected, 'Unexpected reset payload'
        start = int(write['frame.number'])
        end = int(targets[i+1]['frame.number']) if i+1 < len(targets) else float('inf')
        # A reset reply can arrive after unrelated 20 01 00 requests. Match its
        # header and next same-action boundary, not just the next OUT packet.
        replies = [r for r in reads if start < int(r['frame.number']) < end
                   and bytes.fromhex(payload(r))[:3] == data[:3]]
        assert len(replies) == 1, 'Missing or ambiguous settings/reset acknowledgement'
        reply = replies[0]
        assert reply['usb.urb_status'] == '0', 'Failed acknowledgement read'
        paired = reply['frame.number'] in paired_reads
        assert paired or i == 0, 'Only the initial read may predate capture'
        response = packet(reply)
        expected_reply = bytearray(32)
        expected_reply[:3] = data[:3]
        expected_reply[31] = sum(expected_reply[:31]) % 256
        assert response == expected_reply, 'Unexpected acknowledgement contents'
        interleaved = []
        for other in writes:
            if start < int(other['frame.number']) < int(reply['frame.number']):
                assert packet(other)[:3] == bytes.fromhex('200100'), 'Unexpected interleaved action'
                interleaved.append(dict(frame=int(other['frame.number']), write=payload(other),
                                        completion_frame=int(completed[other['frame.number']]['frame.number']),
                                        after_ms=round((float(other['frame.time_relative']) - float(write['frame.time_relative'])) * 1000, 3)))
        tx = dict(write_frame=start, completion_frame=int(completed[write['frame.number']]['frame.number']),
                  write=data.hex(), reply_frame=int(reply['frame.number']), reply=response.hex(),
                  read_submission_captured=paired,
                  reply_ms=round((float(reply['frame.time_relative']) - float(write['frame.time_relative'])) * 1000, 3),
                  interleaved_requests=interleaved)
        if kind in STATES:
            tx.update(zip(['level', 'sleep', 'disable_win', 'disable_alt_f4', 'disable_alt_tab'], STATES[kind][i]))
        transactions.append(tx)
    return dict(sha256=sha256, kind=kind, bus=int(bus), device=int(device),
                total_successful_writes=len(writes), verified_action_replies=len(transactions),
                transactions=transactions)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path)
    parser.add_argument('--kind', choices=OPCODES, required=True)
    parser.add_argument('--bus', type=int, required=True)
    parser.add_argument('--device', type=int, required=True)
    parser.add_argument('--output-dir', type=pathlib.Path, default=pathlib.Path('capture-output'))
    args = parser.parse_args()
    command = ['tshark', '-r', str(args.capture), '-T', 'fields', '-E', 'header=y']
    for field in FIELDS:
        command.extend(['-e', field])
    rows = list(csv.DictReader(io.StringIO(subprocess.check_output(command, text=True)), delimiter='\t'))
    review = review_capture(rows, hashlib.sha256(args.capture.read_bytes()).hexdigest(), args.kind, args.bus, args.device)
    review['capture'] = args.capture.name
    args.output_dir.mkdir(parents=True, exist_ok=True)
    target = args.output_dir / (args.capture.stem + '-review.json')
    target.write_text(json.dumps(review, indent=2) + '\n')
    print(f"{args.kind}: {review['verified_action_replies']} acknowledged actions -> {target}")


if __name__ == '__main__':
    main()
