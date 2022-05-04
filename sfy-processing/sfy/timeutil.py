import pytz

def utcify(dt):
    """
    Assign UTC timezone if tz info is missing
    """
    if dt.tzinfo is None:
        return pytz.utc.localize(dt)
    else:
        return dt

