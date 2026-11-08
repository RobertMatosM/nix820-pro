import copy
import json
import pathlib
import unittest

from wireless_capture import review_capture

FIXTURE = pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures/wireless-lighting-fixtures.json'


def rows():
    sample = json.loads(FIXTURE.read_text())['captures'][0]['transactions'][0]
    result = []
    # Reused URB IDs across two commands must pair with the next completion.
    for offset in (0, 10):
        for frame, urb, kind, endpoint, data, status in [
            (1, 'read', "'S'", '0x85', '', '-115'),
            (2, 'write', "'S'", '0x05', sample['write'], '-115'),
            (3, 'write', "'C'", '0x05', '', '0'),
            (4, 'read', "'C'", '0x85', sample['reply'], '0'),
        ]:
            number = frame + offset
            result.append(dict(zip(
                ['frame.number', 'frame.time_relative', 'usb.bus_id', 'usb.device_address',
                 'usb.urb_id', 'usb.urb_type', 'usb.transfer_type', 'usb.endpoint_address',
                 'usb.urb_status', 'usbhid.data', 'usb.capdata'],
                [str(number), str(number / 1000), '1', '4', urb, kind, '0x01', endpoint,
                 status, data, ''])))
    return result


class WirelessCapture(unittest.TestCase):
    def test_valid_pairs_reused_urbs_and_initial_pending_read(self):
        for source in (rows(), rows()[1:]):
            result = review_capture(source, 'a' * 64, 1, 4)
            self.assertEqual(len(result['transactions']), 2)
            self.assertEqual(result['successful_writes'], 2)
            self.assertEqual(result['transactions'][1]['completion_frame'], 13)
        self.assertFalse(review_capture(rows()[1:], 'a' * 64, 1, 4)['transactions'][0]['read_submission_captured'])

    def test_missing_failed_or_cross_device_completions_are_rejected(self):
        for index, key, value in [(2, 'usb.urb_status', '-1'), (2, 'usb.device_address', '5'),
                                  (2, 'usb.endpoint_address', '0x06'), (3, 'usb.urb_status', '-1'),
                                  (3, 'usb.device_address', '5'), (4, 'usb.device_address', '5')]:
            source = rows()
            source[index][key] = value
            with self.subTest(index=index, key=key), self.assertRaises(AssertionError):
                review_capture(source, 'a' * 64, 1, 4)
        with self.assertRaises(AssertionError):
            review_capture([r for i, r in enumerate(rows()) if i != 2], 'a' * 64, 1, 4)

    def test_checksum_header_body_and_unknown_reply_status_are_rejected(self):
        for index, byte, value, repair in [(1, 31, 0, False), (3, 31, 0, False),
                                         (1, 0, 9, True), (1, 19, 1, True),
                                         (3, 0, 9, True), (3, 3, 255, True)]:
            source = rows()
            data = bytearray.fromhex(source[index]['usbhid.data'])
            data[byte] = value
            if repair:
                data[31] = sum(data[:31]) % 256
            source[index]['usbhid.data'] = data.hex()
            with self.subTest(index=index, byte=byte), self.assertRaises(AssertionError):
                review_capture(source, 'a' * 64, 1, 4)

    def test_unrelated_devices_are_excluded_and_ambiguous_replies_rejected(self):
        source = rows()
        unrelated = copy.deepcopy(source)
        for row in unrelated:
            row['usb.device_address'] = '99'
        self.assertEqual(len(review_capture(source + unrelated, 'a' * 64, 1, 4)['transactions']), 2)
        duplicate = copy.deepcopy(source[3])
        duplicate['frame.number'] = '5'
        source.insert(4, duplicate)
        with self.assertRaises(AssertionError):
            review_capture(source, 'a' * 64, 1, 4)


if __name__ == '__main__':
    unittest.main()
