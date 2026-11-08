import copy
import json
import pathlib
import unittest

from wireless_remap_capture import review_capture

FIXTURE = pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures/wireless-remap-review.json'


def sample(layer, sequence='basic'):
    path = FIXTURE if sequence == 'basic' else FIXTURE.with_name('wireless-remap-followup-review.json')
    return next(c for c in json.loads(path.read_text())['captures'] if c['layer'] == layer)


def rows(layer, sequence='basic'):
    result = []
    def row(frame, urb, kind, endpoint, data='', status='0'):
        return dict(zip(['frame.number', 'frame.time_relative', 'usb.bus_id', 'usb.device_address',
                         'usb.urb_id', 'usb.urb_type', 'usb.transfer_type', 'usb.endpoint_address',
                         'usb.urb_status', 'usbhid.data', 'usb.capdata'],
                        [str(frame), str(frame / 1000), '1', '4', urb, kind, '0x01', endpoint, status, data, '']))
    for tx in sample(layer, sequence)['transactions']:
        frame = tx['write_frame']
        result.extend([row(frame - 1, 'read', "'S'", '0x85', status='-115'),
                       row(frame, 'write', "'S'", '0x05', tx['write'], '-115'),
                       row(tx['completion_frame'], 'write', "'C'", '0x05')])
        if tx['acknowledged']:
            result.append(row(tx['reply_frame'], 'read', "'C'", '0x85', tx['reply']))
        for extra in tx['unrelated_inputs']:
            result.append(row(extra['frame'], 'read', "'C'", '0x85', extra['header'] + '00' * 29))
    return sorted(result, key=lambda r: int(r['frame.number']))


class WirelessRemapCapture(unittest.TestCase):
    def test_fn_pairs_and_top_restore_gap_are_preserved(self):
        for layer, count in [('Top', 1), ('Fn', 2)]:
            result = review_capture(rows(layer), 'a' * 64, layer, 1, 4)
            self.assertEqual(result['verified_replies'], count)
            self.assertEqual([t['record'] for t in result['transactions']],
                             ['02000500', '02000400'] if layer == 'Top' else ['02004900', '02000c00'])
        result = review_capture(rows('Top'), 'a' * 64, 'Top', 1, 4)
        restore = result['transactions'][1]
        self.assertFalse(restore['acknowledged'])
        self.assertIsNone(restore['reply'])
        self.assertEqual(restore['unrelated_inputs'][0]['header'], '55fc04')

    def test_followup_modifier_media_and_replacement_records_keep_ack_limits(self):
        for layer, count, records in [
            ('Top', 0, ['02010000', '03920100', '02000400']),
            ('Fn', 3, ['02004900', '02000500', '02000c00']),
        ]:
            result = review_capture(rows(layer, 'followup'), 'a'*64, layer, 1, 4, 'followup')
            self.assertEqual(result['verified_replies'], count)
            self.assertEqual([t['record'] for t in result['transactions']], records)
            self.assertEqual([t['action'] for t in result['transactions']],
                             ['remap', 'remap', 'restore_original_key'])
        source = rows('Top', 'followup')
        media = next(r for r in source if r['usbhid.data'].startswith('03043803'))
        data = bytearray.fromhex(media['usbhid.data'])
        data[5] = 0  # Preserve the consumer low byte but lose Calculator's high byte.
        data[31] = sum(data[:31]) % 256
        media['usbhid.data'] = data.hex()
        with self.assertRaises(AssertionError):
            review_capture(source, 'a'*64, 'Top', 1, 4, 'followup')

    def test_background_out_does_not_end_pending_remap_reply_window(self):
        source = rows('Fn', 'followup')
        tx = sample('Fn', 'followup')['transactions'][1]
        write = copy.deepcopy(next(r for r in source if int(r['frame.number']) == tx['write_frame']))
        write.update({'frame.number': str(tx['completion_frame']+1), 'usb.urb_id': 'background',
                      'usbhid.data': '200100'+'00'*28+'21'})
        completion = copy.deepcopy(write)
        completion.update({'frame.number': str(tx['completion_frame']+2), 'usb.urb_type': "'C'",
                           'usb.urb_status': '0', 'usbhid.data': ''})
        source.extend([write, completion])
        source.sort(key=lambda r: int(r['frame.number']))
        result = review_capture(source, 'a'*64, 'Fn', 1, 4, 'followup')
        self.assertEqual(result['verified_replies'], 3)
        self.assertEqual(result['transactions'][1]['reply_frame'], tx['reply_frame'])

    def test_corrupt_writes_sources_records_and_failed_completions_are_rejected(self):
        for byte, value, repair in [(31, 0, False), (2, 1, True), (3, 3, True), (7, 1, True)]:
            source = rows('Fn')
            write = next(r for r in source if r['usb.endpoint_address'] == '0x05' and r['usb.urb_type'] == "'S'")
            data = bytearray.fromhex(write['usbhid.data'])
            data[byte] = value
            if repair:
                data[31] = sum(data[:31]) % 256
            write['usbhid.data'] = data.hex()
            with self.subTest(byte=byte), self.assertRaises(AssertionError):
                review_capture(source, 'a' * 64, 'Fn', 1, 4)
        source = rows('Fn')
        completion = next(r for r in source if r['usb.endpoint_address'] == '0x05' and r['usb.urb_type'] == "'C'")
        completion['usb.urb_status'] = '-1'
        with self.assertRaises(AssertionError):
            review_capture(source, 'a' * 64, 'Fn', 1, 4)

    def test_bad_matching_replies_rejected_and_unrelated_traffic_not_promoted(self):
        for byte in (3, 31):
            source = rows('Fn')
            reply = next(r for r in source if r['usb.endpoint_address'] == '0x85' and r['usb.urb_type'] == "'C'")
            data = bytearray.fromhex(reply['usbhid.data'])
            data[byte] ^= 1
            if byte != 31:
                data[31] = sum(data[:31]) % 256
            reply['usbhid.data'] = data.hex()
            with self.assertRaises(AssertionError):
                review_capture(source, 'a' * 64, 'Fn', 1, 4)
        source = rows('Fn')
        unrelated = copy.deepcopy(source)
        for row in unrelated:
            row['usb.device_address'] = '99'
        self.assertEqual(review_capture(source + unrelated, 'a' * 64, 'Fn', 1, 4)['verified_replies'], 2)
        reply = next(r for r in source if r['usb.endpoint_address'] == '0x85' and r['usb.urb_type'] == "'C'")
        reply['usb.device_address'] = '99'
        self.assertEqual(review_capture(source, 'a' * 64, 'Fn', 1, 4)['verified_replies'], 1)


if __name__ == '__main__':
    unittest.main()
