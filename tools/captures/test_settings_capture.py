"""Settings extraction rejects corrupted writes, acknowledgements and labels."""
import copy
import json
import pathlib
import unittest

from settings_capture import review_capture


class SettingsReview(unittest.TestCase):
    def setUp(self):
        fixture = json.loads(((pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures') / 'system-settings-fixtures.json').read_text())
        self.rows = []

        def add(kind, urb, data='', request='', status='0'):
            self.rows.append({'frame.number': str(len(self.rows) + 1),
                              'frame.time_relative': str(len(self.rows) / 1000),
                              'usb.bus_id': '1', 'usb.device_address': '12',
                              'usb.urb_type': kind, 'usb.urb_id': urb,
                              'usb.urb_status': status, 'usb.bmRequestType': request,
                              'usbhid.setup.bRequest': '0x09' if request == '0x21' else '0x01',
                              'usbhid.setup.wIndex': '3', 'usbhid.setup.wValue': '0x0300',
                              'usbhid.data': data})
        for tx in fixture['transactions']:
            replies = iter(tx['replies'])
            for i, write in enumerate(tx['writes']):
                add("'S'", 'reused-write', write, '0x21')
                add("'C'", 'reused-write')
                if i in (0, 1, 3):
                    add("'S'", 'reused-read', request='0xa1')
                    add("'C'", 'reused-read', next(replies)['payload'])

    def test_all_labelled_fields_and_changes(self):
        review, txs, reports = review_capture(self.rows, 'test')
        self.assertEqual(review['successful_completions'], 48)
        self.assertEqual(review['verified_command_replies'], 36)
        self.assertEqual(review['unique_settings'], 11)
        self.assertEqual(len(reports), 48)
        self.assertIsNone(txs[0]['changed_offsets'])
        self.assertEqual([t['changed_offsets'] for t in txs[1:]], [[8]] * 4 + [[6]] * 4 + [[4], [3], [2]])
        self.assertFalse(review['reset_commands_captured'])

    def test_failed_missing_or_cross_device_completions_are_rejected(self):
        for kind in ('failed', 'missing', 'wrong-device'):
            with self.subTest(kind=kind):
                rows = copy.deepcopy(self.rows)
                if kind == 'missing':
                    del rows[1]
                elif kind == 'failed':
                    rows[1]['usb.urb_status'] = '-32'
                else:
                    rows[1]['usb.device_address'] = '99'
                with self.assertRaises(AssertionError):
                    review_capture(rows, 'test')

    def test_corrupt_payload_or_rejected_wrong_or_short_reply_is_rejected(self):
        for row_index, value in [(8, '000100000000010006' + '00' * 53 + 'aa55'),
                                 (3, '04180000' + '00' * 60),
                                 (3, '04170101' + '00' * 60), (3, '04180001')]:
            with self.subTest(row=row_index, value=value):
                rows = copy.deepcopy(self.rows)
                rows[row_index]['usbhid.data'] = value
                with self.assertRaises(AssertionError):
                    review_capture(rows, 'test')


if __name__ == '__main__':
    unittest.main()
