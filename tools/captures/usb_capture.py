"""Shared read-only USB capture helpers. URB addresses are reused in a capture."""


def match_completions(rows):
    """Pair each submission with its next completion, never the last use of an URB."""
    pending = {}
    completed = {}
    for row in rows:
        key = (row.get('usb.bus_id'), row.get('usb.device_address'), row['usb.urb_id'])
        if row['usb.urb_type'] == "'S'":
            pending[key] = row
        elif row['usb.urb_type'] == "'C'" and key in pending:
            submitted = pending.pop(key)
            completed[submitted['frame.number']] = row
    return completed


def payload(row):
    return row.get('usbhid.data') or row.get('usb.capdata') or ''
