"""Reset extraction must retain exact workflows and verified command replies."""
import copy
import json
import pathlib
import unittest

from reset_capture import review_capture


class ResetReview(unittest.TestCase):
    def rows(self, extra_refresh_read=False):
        fixture = json.loads(((pathlib.Path(__file__).resolve().parents[2] / 'tests/fixtures') / 'reset-keyboard-and-factory-reset-fixtures.json').read_text())
        rows = []
        def add(kind, urb, data='', request='', status='0'):
            rows.append({'frame.number':str(len(rows)+1), 'frame.time_relative':str(len(rows)/1000),
                         'usb.bus_id':'1','usb.device_address':'9','usb.urb_type':kind,
                         'usb.urb_id':urb,'usb.urb_status':status,'usb.bmRequestType':request,
                         'usbhid.setup.bRequest':'0x09' if request=='0x21' else '0x01',
                         'usbhid.setup.wIndex':'3','usbhid.setup.wValue':'0x0300','usbhid.data':data})
        for tx in fixture['transactions']:
            replies = iter(tx['replies'])
            for write in tx['writes']:
                add("'S'",'write',write['payload'],'0x21')
                add("'C'",'write')
                if write['read_reply']:
                    add("'S'",'read',request='0xa1')
                    add("'C'",'read',next(replies)['payload'])
                elif extra_refresh_read and write['payload'].startswith('04f0'):
                    add("'S'",'read',request='0xa1')
                    add("'C'",'read','04f00001'+'00'*60)
        return rows

    def test_both_reset_workflows_and_actual_status_bytes_are_retained(self):
        review, fixture, reports = review_capture(self.rows(),'test')
        self.assertEqual(review['successful_completions'],72)
        self.assertEqual(review['verified_command_replies'],25)
        self.assertEqual([len(tx['writes']) for tx in fixture['transactions']],[26,46])
        self.assertEqual(len(reports),72)
        self.assertFalse(review['factory_lighting_refresh_read'])
        self.assertTrue(any(r['payload'].startswith('04020001ff') for tx in fixture['transactions'] for r in tx['replies']))

    def test_failed_missing_or_cross_device_completions_rejected(self):
        for kind in ('failed','missing','cross-device'):
            with self.subTest(kind=kind):
                rows=self.rows()
                if kind=='failed': rows[1]['usb.urb_status']='-32'
                elif kind=='missing': del rows[1]
                else: rows[1]['usb.device_address']='99'
                with self.assertRaises(AssertionError): review_capture(rows,'test')

    def test_changed_clear_data_commands_or_failed_replies_rejected(self):
        for kind in ('data','command','rejected','prefix','missing-tail'):
            with self.subTest(kind=kind):
                rows=self.rows()
                if kind=='missing-tail': rows=rows[:-1]
                elif kind=='data': rows[8]['usbhid.data']='01'+rows[8]['usbhid.data'][2:]
                elif kind=='command': rows[4]['usbhid.data']='0427'+rows[4]['usbhid.data'][4:]
                else:
                    data=bytearray.fromhex(rows[3]['usbhid.data'])
                    data[3 if kind=='rejected' else 1]=0xff
                    rows[3]['usbhid.data']=data.hex()
                with self.assertRaises(AssertionError): review_capture(rows,'test')

    def test_factory_lighting_refresh_does_not_invent_a_reply_read(self):
        with self.assertRaises(AssertionError): review_capture(self.rows(extra_refresh_read=True),'test')


if __name__ == '__main__':
    unittest.main()
