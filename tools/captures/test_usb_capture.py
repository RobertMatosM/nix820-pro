"""Regression coverage for USB address reuse; no device access."""
import unittest
from usb_capture import match_completions


class CompletionMatching(unittest.TestCase):
    def test_reused_urb_keeps_each_completion_and_its_status(self):
        rows = [dict(zip(('frame.number', 'usb.urb_id', 'usb.urb_type', 'usb.urb_status'), row))
                for row in [('1', 'same', "'S'", '-115'),
                            ('2', 'same', "'C'", '-32'),
                            ('3', 'same', "'S'", '-115'),
                            ('4', 'same', "'C'", '0'),
                            ('5', 'same', "'S'", '-115')]]
        paired = match_completions(rows)
        self.assertEqual(paired['1']['frame.number'], '2')
        self.assertEqual(paired['1']['usb.urb_status'], '-32')
        self.assertEqual(paired['3']['frame.number'], '4')
        self.assertNotIn('5', paired)

    def test_devices_and_buses_cannot_share_a_completion(self):
        rows = []
        for frame, bus, device, kind in [('1', '1', '11', "'S'"),
                                         ('2', '1', '12', "'C'"),
                                         ('3', '2', '11', "'C'"),
                                         ('4', '1', '11', "'C'")]:
            rows.append({'frame.number': frame, 'usb.bus_id': bus,
                         'usb.device_address': device, 'usb.urb_id': 'same',
                         'usb.urb_type': kind})
        self.assertEqual(match_completions(rows)['1']['frame.number'], '4')


if __name__ == '__main__':
    unittest.main()
