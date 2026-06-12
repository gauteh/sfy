import numpy as np
from sfy import event
from datetime import datetime, timezone
from . import *


def test_parse_track():
    d = open(
        'tests/data/1642676373013-b52920fb-eef5-4796-8e38-198a56741379__track.qo.json'

    ).read()
    a = event.Event.parse(d)
    print(a)

def test_parse_log():
    d = open(
        'tests/data/1642676372928-a9b6422f-cea5-4e4c-8b56-248495756115__log.qo.json'

    ).read()
    a = event.Event.parse(d)
    print(a)

@needs_hub
def test_position_range(sfyhub, tmpdir):
    b = sfyhub.buoy('bug08')
    pcks = b.position_packages_range(
        datetime(2022, 8, 14, 00, 00, tzinfo=timezone.utc),
        datetime(2022, 8, 15, 23, 59, tzinfo=timezone.utc))

    pos = np.array([[p.best_position_time, p.longitude, p.latitude, p.position_type, p.file] for p in pcks])
    print(pos)
    print(pos.shape)

    assert len(pos) > 2000
    assert any('track' in t for t in pos[:,4])
    assert any('axl.qo' in t for t in pos[:,4])

@needs_hub
def test_parse_session_qo_open(sfyhub):
    """Parse real opening _session.qo (session.begin) events fetched from the hub."""
    from datetime import timedelta
    b = sfyhub.buoy('sfy4-01')
    end = datetime.now(tz=timezone.utc)
    start = end - timedelta(hours=24)

    pcks = b.position_packages_range(start, end)
    opening = [p for p in pcks if p.file == '_session.qo' and p.req == 'session.begin']

    print(f'Found {len(opening)} opening _session.qo packages')
    assert len(opening) > 0
    assert all(p.voltage is not None for p in opening)
    assert all(p.get_voltage() is not None for p in opening)

@needs_hub
def test_parse_session_qo_close(sfyhub):
    """Parse closing _session.qo (session.end) events that carry hub_* fields."""
    from datetime import timedelta
    b = sfyhub.buoy('sfy4-01')
    end = datetime.now(tz=timezone.utc)
    start = end - timedelta(hours=24)

    pcks = b.position_packages_range(start, end)
    closing = [p for p in pcks if p.file == '_session.qo' and p.req == 'session.end']

    print(f'Found {len(closing)} closing _session.qo packages')
    assert len(closing) > 0
    assert all(p.hub_last_work_done is not None for p in closing)

@needs_hub
def test_parse_session_qo(sfyhub):
    from datetime import timedelta
    b = sfyhub.buoy('sfy4-01')
    end = datetime.now(tz=timezone.utc)
    start = end - timedelta(hours=24)

    pcks = b.position_packages_range(start, end)
    session_pcks = [p for p in pcks if p.file == '_session.qo']

    print(f'Found {len(session_pcks)} _session.qo packages out of {len(pcks)} total')
    for p in session_pcks:
        print(p.file, p.best_position_time, p.longitude, p.latitude, p.get_voltage())

    assert len(session_pcks) > 0
    assert any(p.get_voltage() is not None for p in session_pcks)

