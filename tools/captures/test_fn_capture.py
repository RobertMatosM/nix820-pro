"""Checks for Fn command identity, labelled I/Insert records and transaction evidence."""
import copy
import json
import pathlib
import unittest

from fn_capture import review_fn_capture


class FnReview(unittest.TestCase):
    def setUp(self):
        fixture = json.loads(((pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures') /
                              'fn-layer-remap-test-fixtures.json').read_text())
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

    def test_labelled_insert_and_observed_clear_keep_actual_fn_commands(self):
        review, fixture, reports = review_fn_capture(self.rows, 'test')
        self.assertEqual(review['commands'], ['0427'])
        self.assertEqual(review['source_slot'], 45)
        self.assertEqual(review['successful_completions'], 26)
        self.assertEqual(review['verified_command_replies'], 8)
        self.assertEqual(len(reports), 26)
        self.assertEqual(fixture['transactions'][0]['nonzero_slots'], [45])
        self.assertEqual(fixture['transactions'][1]['nonzero_slots'], [])
        self.assertFalse(review['physical_key_output_verified'])

    def test_top_command_is_rejected_even_with_matching_reply(self):
        rows = copy.deepcopy(self.rows)
        for index in (4, 7):
            data = bytearray.fromhex(rows[index]['usbhid.data'])
            data[1] = 0x11
            rows[index]['usbhid.data'] = data.hex()
        with self.assertRaises(AssertionError):
            review_fn_capture(rows, 'test')

    def test_wrong_source_destination_or_incomplete_capture_is_rejected(self):
        # Third data block contains source slot 45 at byte 52.
        for kind in ('source', 'destination', 'missing-clear'):
            with self.subTest(kind=kind):
                rows = copy.deepcopy(self.rows)
                if kind == 'missing-clear':
                    rows = rows[:34]
                else:
                    data = bytearray.fromhex(rows[12]['usbhid.data'])
                    if kind == 'source':
                        data[56:60], data[52:56] = data[52:56], bytes(4)
                    else:
                        data[54] = 0x4a
                    rows[12]['usbhid.data'] = data.hex()
                with self.assertRaises(AssertionError):
                    review_fn_capture(rows, 'test')

    def test_missing_failed_or_cross_device_completion_and_bad_replies_are_rejected(self):
        for kind in ('missing', 'failed', 'cross-device', 'prefix', 'negative'):
            with self.subTest(kind=kind):
                rows = copy.deepcopy(self.rows)
                if kind == 'missing':
                    del rows[1]
                elif kind == 'failed':
                    rows[1]['usb.urb_status'] = '-32'
                elif kind == 'cross-device':
                    rows[1]['usb.device_address'] = '99'
                else:
                    data = bytearray.fromhex(rows[7]['usbhid.data'])
                    data[1 if kind == 'prefix' else 3] = 0xff
                    rows[7]['usbhid.data'] = data.hex()
                with self.assertRaises(AssertionError):
                    review_fn_capture(rows, 'test')


if __name__ == '__main__':
    unittest.main()
