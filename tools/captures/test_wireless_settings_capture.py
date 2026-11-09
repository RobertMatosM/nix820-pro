import copy
import json
import pathlib
import unittest

from wireless_settings_capture import review_capture

FIXTURE = pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures/wireless-settings-review.json'


def sample(kind):
    return next(c for c in json.loads(FIXTURE.read_text())['captures'] if c['kind'] == kind)


def rows(kind):
    result = []
    def row(frame, time, urb, typ, endpoint, data='', status='0'):
        return dict(zip(['frame.number', 'frame.time_relative', 'usb.bus_id', 'usb.device_address',
                         'usb.urb_id', 'usb.urb_type', 'usb.transfer_type', 'usb.endpoint_address',
                         'usb.urb_status', 'usbhid.data', 'usb.capdata'],
                        [str(frame), str(time), '1', '4', urb, typ, '0x01', endpoint, status, data, '']))
    for index, tx in enumerate(sample(kind)['transactions']):
        time = index * 10
        frame = tx['write_frame']
        result.extend([row(frame-1, time-.01, 'read', "'S'", '0x85', status='-115'),
                       row(frame, time, 'write', "'S'", '0x05', tx['write'], '-115'),
                       row(tx['completion_frame'], time+.001, 'write', "'C'", '0x05'),
                       row(tx['reply_frame'], time+tx['reply_ms']/1000, 'read', "'C'", '0x85', tx['reply'])])
        for query in tx['interleaved_requests']:
            result.extend([row(query['frame'], time+query['after_ms']/1000, 'write', "'S'", '0x05', query['write'], '-115'),
                           row(query['completion_frame'], time+query['after_ms']/1000+.001, 'write', "'C'", '0x05')])
    return sorted(result, key=lambda r: int(r['frame.number']))


class WirelessSettingsCapture(unittest.TestCase):
    def test_all_labelled_actions_and_delayed_reset_reply_are_preserved(self):
        for kind, count in [('levels', 5), ('sleep', 5), ('locks', 6), ('reset-keyboard', 1), ('factory-reset', 1)]:
            result = review_capture(rows(kind), 'a'*64, kind, 1, 4)
            self.assertEqual(result['verified_action_replies'], count)
        reset = review_capture(rows('reset-keyboard'), 'a'*64, 'reset-keyboard', 1, 4)['transactions'][0]
        self.assertEqual(reset['reply_ms'], 1335.558)
        self.assertEqual(len(reset['interleaved_requests']), 2)
        self.assertGreater(reset['reply_frame'], reset['interleaved_requests'][-1]['frame'])
        self.assertEqual(reset['reply'][:6], '1f0000')

    def test_failed_or_missing_writes_and_malformed_payloads_are_rejected(self):
        for byte, value, repair in [(11, 5, True), (17, 0, True), (31, 0, False)]:
            source = rows('levels')
            write = next(r for r in source if r['usb.urb_type'] == "'S'" and r['usb.endpoint_address'] == '0x05')
            data = bytearray.fromhex(write['usbhid.data'])
            data[byte] = value
            if repair:
                data[31] = sum(data[:31]) % 256
            write['usbhid.data'] = data.hex()
            with self.assertRaises(AssertionError):
                review_capture(source, 'a'*64, 'levels', 1, 4)
        source = rows('factory-reset')
        completion = next(r for r in source if r['usb.urb_type'] == "'C'" and r['usb.endpoint_address'] == '0x05')
        completion['usb.urb_status'] = '-1'
        with self.assertRaises(AssertionError):
            review_capture(source, 'a'*64, 'factory-reset', 1, 4)

    def test_bad_or_cross_device_replies_are_never_accepted(self):
        for field, value in [('usb.device_address', '99'), ('usb.urb_status', '-1')]:
            source = rows('reset-keyboard')
            reply = next(r for r in source if r['usb.urb_type'] == "'C'" and r['usb.endpoint_address'] == '0x85')
            reply[field] = value
            with self.assertRaises(AssertionError):
                review_capture(source, 'a'*64, 'reset-keyboard', 1, 4)
        source = rows('factory-reset')
        reply = next(r for r in source if r['usb.urb_type'] == "'C'" and r['usb.endpoint_address'] == '0x85')
        data = bytearray.fromhex(reply['usbhid.data'])
        data[3] = 255
        data[31] = sum(data[:31]) % 256
        reply['usbhid.data'] = data.hex()
        with self.assertRaises(AssertionError):
            review_capture(source, 'a'*64, 'factory-reset', 1, 4)
        source = rows('locks')
        unrelated = copy.deepcopy(source)
        for row in unrelated:
            row['usb.device_address'] = '99'
        self.assertEqual(review_capture(source+unrelated, 'a'*64, 'locks', 1, 4)['verified_action_replies'], 6)


if __name__ == '__main__':
    unittest.main()
