"""Regressions for the labelled Esc media sweep and its strict action validation."""
import copy
import json
import pathlib
import unittest

from media_capture import ACTIONS, review_media_capture


class MediaReview(unittest.TestCase):
    def setUp(self):
        fixture = json.loads(((pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures') /
                              'media-capture-remap-test-fixtures.json').read_text())
        self.rows = []

        def add(kind, urb, data='', request='', status='0'):
            self.rows.append({'frame.number': str(len(self.rows) + 1),
                              'frame.time_relative': str(len(self.rows) / 1000),
                              'usb.bus_id': '1', 'usb.device_address': '9',
                              'usb.urb_type': kind, 'usb.urb_id': urb,
                              'usb.urb_status': status, 'usb.bmRequestType': request,
                              'usbhid.setup.bRequest': '0x09' if request == '0x21' else '0x01',
                              'usbhid.setup.wIndex': '3', 'usbhid.setup.wValue': '0x0300',
                              'usbhid.data': data})
        for tx in fixture['transactions']:
            replies = iter(tx['replies'])
            for index, write in enumerate(tx['writes']):
                add("'S'", 'reused-write', write['payload'], '0x21')
                add("'C'", 'reused-write')
                if index in (0, 1, 11, 12):
                    add("'S'", 'reused-read', request='0xa1')
                    add("'C'", 'reused-read', next(replies)['payload'])

    def test_all_18_operator_actions_and_own_command_replies(self):
        review, fixture, reports = review_media_capture(self.rows, 'test')
        self.assertEqual(review['successful_completions'], 234)
        self.assertEqual(review['verified_command_replies'], 72)
        self.assertEqual(review['media_destination_count'], 18)
        self.assertEqual(review['source_slot'], 1)
        self.assertEqual(len(reports), 234)
        self.assertEqual([tx['record'] for tx in fixture['transactions']], [a[2] for a in ACTIONS])
        self.assertFalse(review['default_restore_captured'])

    def test_changed_source_order_or_unrelated_records_are_rejected(self):
        # First data report starts after the two acknowledged setup commands.
        for kind in ('wrong-source', 'wrong-action', 'extra-record', 'missing-action'):
            with self.subTest(kind=kind):
                rows = copy.deepcopy(self.rows)
                if kind == 'missing-action':
                    rows = rows[:-34]
                else:
                    data = bytearray.fromhex(rows[8]['usbhid.data'])
                    if kind == 'wrong-source':
                        data[8:12], data[4:8] = data[4:8], bytes(4)
                    elif kind == 'wrong-action':
                        data[4:8] = bytes.fromhex(ACTIONS[1][2])
                    else:
                        data[8:12] = data[4:8]
                    rows[8]['usbhid.data'] = data.hex()
                with self.assertRaises(AssertionError):
                    review_media_capture(rows, 'test')

    def test_failed_cross_device_writes_and_negative_or_mismatched_replies_are_rejected(self):
        for kind in ('failed-write', 'wrong-device', 'rejected', 'wrong-prefix'):
            with self.subTest(kind=kind):
                rows = copy.deepcopy(self.rows)
                if kind == 'failed-write':
                    rows[1]['usb.urb_status'] = '-32'
                elif kind == 'wrong-device':
                    rows[1]['usb.device_address'] = '99'
                else:
                    data = bytearray.fromhex(rows[3]['usbhid.data'])
                    data[3 if kind == 'rejected' else 1] = 0xff
                    rows[3]['usbhid.data'] = data.hex()
                with self.assertRaises(AssertionError):
                    review_media_capture(rows, 'test')


if __name__ == '__main__':
    unittest.main()
