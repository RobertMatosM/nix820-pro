"""Extract only scoped 2.4 GHz lighting requests/replies; never opens a keyboard."""
import argparse
import csv
import hashlib
import io
import json
import pathlib
import subprocess

from usb_capture import match_completions, payload

FIELDS = ['frame.number', 'frame.time_relative', 'usb.bus_id', 'usb.device_address',
          'usb.urb_id', 'usb.urb_type', 'usb.transfer_type', 'usb.endpoint_address',
          'usb.urb_status', 'usbhid.data', 'usb.capdata']


def packet(row):
    data = bytes.fromhex(payload(row))
    assert len(data) == 32, 'Expected a 32-byte receiver packet'
    assert sum(data[:31]) % 256 == data[31], 'Invalid receiver checksum'
    return data


def review_capture(rows, sha256, bus, device):
    scoped = [r for r in rows if r['usb.bus_id'] == str(bus)
              and r['usb.device_address'] == str(device)]
    completed = match_completions(scoped)
    paired_reads = {r['frame.number'] for r in completed.values()}
    writes = [r for r in scoped if r['usb.transfer_type'] == '0x01'
              and r['usb.endpoint_address'] == '0x05' and r['usb.urb_type'] == "'S'"]
    reads = [r for r in scoped if r['usb.transfer_type'] == '0x01'
             and r['usb.endpoint_address'] == '0x85' and r['usb.urb_type'] == "'C'"
             and payload(r)]
    assert writes, 'No receiver writes in the selected bus/device'
    transactions = []
    for i, write in enumerate(writes):
        frame = int(write['frame.number'])
        end = int(writes[i + 1]['frame.number']) if i + 1 < len(writes) else float('inf')
        completion = completed.get(write['frame.number'], {})
        assert completion.get('usb.urb_status') == '0', 'Failed or missing write completion'
        assert completion.get('usb.endpoint_address') == '0x05'
        assert frame < int(completion['frame.number']) < end
        replies = [r for r in reads if frame < int(r['frame.number']) < end]
        assert len(replies) == 1, 'Missing or ambiguous receiver reply'
        reply = replies[0]
        assert reply['usb.urb_status'] == '0', 'Failed receiver read'
        paired = reply['frame.number'] in paired_reads
        assert paired or i == 0, 'Only the initial read may predate capture'
        data, response = packet(write), packet(reply)
        assert data[:3] == response[:3], 'Reply header does not match command'
        if data[:3] == bytes.fromhex('200100'):
            continue  # Unknown non-lighting command; do not publish it as a lighting fixture.
        assert data[:3] == bytes.fromhex('051000'), 'Unexpected receiver command'
        assert 0 <= data[3] <= 19 and data[11] in (0, 1)
        assert data[7:11] == bytes(4) and data[14:17] == bytes(3)
        assert data[17:19] == bytes.fromhex('aa55') and data[19:31] == bytes(12)
        if data[3]:
            assert 1 <= data[12] <= 5 and 1 <= data[13] <= 5
        else:
            assert data[3:17] == bytes(14)
        assert response == bytes.fromhex('051000' + '00' * 28 + '15'), 'Unrecognized lighting reply'
        transactions.append(dict(write_frame=frame, completion_frame=int(completion['frame.number']),
                                 reply_frame=int(reply['frame.number']), read_submission_captured=paired,
                                 write=data.hex(), reply=response.hex(),
                                 reply_ms=round((float(reply['frame.time_relative']) -
                                                 float(write['frame.time_relative'])) * 1000, 3)))
    assert len(reads) == len(writes), 'Unmatched receiver replies'
    assert transactions, 'No lighting transactions'
    return dict(sha256=sha256, bus=int(bus), device=int(device),
                successful_writes=len(writes), matching_replies=len(reads), transactions=transactions)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path)
    parser.add_argument('--bus', type=int, required=True)
    parser.add_argument('--device', type=int, required=True)
    parser.add_argument('--output-dir', type=pathlib.Path, default=pathlib.Path('capture-output'))
    args = parser.parse_args()
    command = ['tshark', '-r', str(args.capture), '-T', 'fields', '-E', 'header=y']
    for field in FIELDS:
        command.extend(['-e', field])
    rows = list(csv.DictReader(io.StringIO(subprocess.check_output(command, text=True)), delimiter='\t'))
    review = review_capture(rows, hashlib.sha256(args.capture.read_bytes()).hexdigest(), args.bus, args.device)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    target = args.output_dir / (args.capture.stem + '-fixtures.json')
    target.write_text(json.dumps(review, indent=2) + '\n')
    print(f"{len(review['transactions'])} verified lighting pairs -> {target}")


if __name__ == '__main__':
    main()
