"""Negative transfer/reply checks for offline source-sweep review."""
import copy
import json
import pathlib
import unittest

from physical_capture import review_capture


class PhysicalReview(unittest.TestCase):
    def setUp(self):
        root = (pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures')
        body = bytes.fromhex(json.loads((root / 'exclusive-config-physical-test-fixtures.json').read_text())['tables'][0]['body'])
        commands = []
        for op in (0x18, 0x11, 0x02, 0xf0):
            command = bytearray(64)
            command[:2] = bytes([4, op])
            if op == 0x11:
                command[8] = 9
            commands.append(bytes(command))
        writes = commands[:2] + [body[i:i + 64] for i in range(0, 576, 64)] + commands[2:]
        self.rows = []

        def add(kind, urb, data, request='', status='0'):
            self.rows.append({'frame.number': str(len(self.rows) + 1),
                              'frame.time_relative': str(len(self.rows) / 1000),
                              'usb.bus_id': '1', 'usb.device_address': '11',
                              'usb.urb_type': kind, 'usb.urb_id': urb,
                              'usb.urb_status': status,
                              'usb.bmRequestType': request,
                              'usbhid.setup.bRequest': '0x09' if request == '0x21' else '0x01',
                              'usbhid.setup.wIndex': '3',
                              'usbhid.setup.wValue': '0x0300', 'usbhid.data': data.hex()})
        for i, write in enumerate(writes):
            add("'S'", 'write', write, '0x21')
            add("'C'", 'write', b'')
            if i in (0, 1, 11, 12):
                reply = bytearray(write)
                reply[3] = 1
                add("'S'", 'read', b'', '0xa1')
                add("'C'", 'read', reply)

    def test_first_table_does_not_invent_a_predecessor(self):
        review, tables, _ = review_capture(self.rows, 'test')
        self.assertEqual(review['source_slot_count'], 1)
        self.assertIsNone(tables[0]['changed_slots'])
        self.assertEqual(review['verified_command_replies'], 4)
        self.assertFalse(review['physical_labels_confirmed'])

    def test_failed_write_and_missing_completion_are_rejected(self):
        for missing in (False, True):
            with self.subTest(missing=missing):
                rows = copy.deepcopy(self.rows)
                if missing:
                    del rows[1]
                else:
                    rows[1]['usb.urb_status'] = '-32'
                with self.assertRaises(AssertionError):
                    review_capture(rows, 'test')

    def test_wrong_prefix_and_rejected_reply_are_rejected(self):
        for data in ('04130101', '04180000'):
            with self.subTest(data=data):
                rows = copy.deepcopy(self.rows)
                rows[3]['usbhid.data'] = data + '00' * 60
                with self.assertRaises(AssertionError):
                    review_capture(rows, 'test')


if __name__ == '__main__':
    unittest.main()
