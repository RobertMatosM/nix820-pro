"""Review the labelled wireless remap and modifier/media follow-up experiments."""
import argparse
import csv
import hashlib
import io
import json
import pathlib
import subprocess

from usb_capture import match_completions, payload
from wireless_capture import FIELDS, packet

EXPERIMENTS = {
    'Top': dict(op=0x03, source='A', slot=56, records=['02000500', '02000400']),
    'Fn': dict(op=0x0d, source='I', slot=45, records=['02004900', '02000c00']),
}
FOLLOWUPS = {
    'Top': dict(op=0x03, source='A', slot=56, records=['02010000', '03920100', '02000400']),
    'Fn': dict(op=0x0d, source='I', slot=45, records=['02004900', '02000500', '02000c00']),
}


def review_capture(rows, sha256, layer, bus, device, sequence='basic'):
    assert sequence in ('basic', 'followup'), 'Unsupported capture sequence'
    experiment = (EXPERIMENTS if sequence == 'basic' else FOLLOWUPS)[layer]
    scoped = [r for r in rows if r['usb.bus_id'] == str(bus)
              and r['usb.device_address'] == str(device)]
    completed = match_completions(scoped)
    paired_reads = {r['frame.number'] for r in completed.values()}
    writes = [r for r in scoped if r['usb.urb_type'] == "'S'"
              and r['usb.transfer_type'] == '0x01' and r['usb.endpoint_address'] == '0x05']
    reads = [r for r in scoped if r['usb.urb_type'] == "'C'"
             and r['usb.transfer_type'] == '0x01' and r['usb.endpoint_address'] == '0x85'
             and payload(r)]
    transactions = []
    for i, write in enumerate(writes):
        data = bytes.fromhex(payload(write))
        if not data or data[0] != experiment['op']:
            continue  # Extra commands are not remap evidence.
        data = packet(write)
        assert data[:3] == bytes([experiment['op'], 4, experiment['slot']]), 'Wrong layer/source header'
        index = len(transactions)
        assert index < len(experiment['records']), 'Unexpected extra remap'
        assert data[3:7].hex() == experiment['records'][index], 'Unexpected destination/default record'
        assert data[7:31] == bytes(24), 'Nonzero remap padding'
        frame = int(write['frame.number'])
        # Other queries can occur while a remap reply is pending. Bound by
        # the next request with this layer/source header, not any outgoing write.
        next_same = next((other for other in writes[i+1:]
                          if bytes.fromhex(payload(other))[:3] == data[:3]), None)
        end = int(next_same['frame.number']) if next_same else float('inf')
        completion = completed.get(write['frame.number'], {})
        assert completion.get('usb.urb_status') == '0', 'Failed or missing write completion'
        assert completion['usb.endpoint_address'] == '0x05'
        assert frame < int(completion['frame.number']) < end
        window = [r for r in reads if frame < int(r['frame.number']) < end]
        matching = [r for r in window if bytes.fromhex(payload(r))[:3] == data[:3]]
        assert len(matching) <= 1, 'Ambiguous remap replies'
        reply_data = None
        reply = matching[0] if matching else None
        if reply:
            assert reply['usb.urb_status'] == '0', 'Failed remap read'
            assert reply['frame.number'] in paired_reads, 'Missing remap read submission'
            response = packet(reply)
            expected = bytearray(32)
            expected[:3] = data[:3]
            expected[31] = sum(expected[:31]) % 256
            assert response == expected, 'Unrecognized remap acknowledgement'
            reply_data = response.hex()
        transactions.append(dict(
            action='restore_original_key' if index == len(experiment['records']) - 1 else 'remap',
            write_frame=frame, completion_frame=int(completion['frame.number']),
            write=data.hex(), record=data[3:7].hex(), acknowledged=reply is not None,
            reply_frame=int(reply['frame.number']) if reply else None, reply=reply_data,
            reply_ms=round((float(reply['frame.time_relative']) - float(write['frame.time_relative'])) * 1000, 3) if reply else None,
            unrelated_inputs=[dict(frame=int(r['frame.number']), header=payload(r)[:6])
                              for r in window if r not in matching]))
    assert len(transactions) == len(experiment['records']), 'Incomplete labelled remap sequence'
    return dict(sha256=sha256, bus=int(bus), device=int(device), layer=layer, sequence=sequence,
                source=experiment['source'], source_slot=experiment['slot'],
                transactions=transactions, verified_replies=sum(t['acknowledged'] for t in transactions))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path)
    parser.add_argument('--layer', choices=EXPERIMENTS, required=True)
    parser.add_argument('--sequence', choices=['basic', 'followup'], default='basic')
    parser.add_argument('--bus', type=int, required=True)
    parser.add_argument('--device', type=int, required=True)
    parser.add_argument('--output-dir', type=pathlib.Path, default=pathlib.Path('capture-output'))
    args = parser.parse_args()
    command = ['tshark', '-r', str(args.capture), '-T', 'fields', '-E', 'header=y']
    for field in FIELDS:
        command.extend(['-e', field])
    rows = list(csv.DictReader(io.StringIO(subprocess.check_output(command, text=True)), delimiter='\t'))
    review = review_capture(rows, hashlib.sha256(args.capture.read_bytes()).hexdigest(), args.layer, args.bus, args.device, args.sequence)
    review['capture'] = args.capture.name
    args.output_dir.mkdir(parents=True, exist_ok=True)
    target = args.output_dir / (args.capture.stem + '-review.json')
    target.write_text(json.dumps(review, indent=2) + '\n')
    print(f"{args.layer}: {len(review['transactions'])} successful remap writes, {review['verified_replies']} matching acknowledgements -> {target}")


if __name__ == '__main__':
    main()
