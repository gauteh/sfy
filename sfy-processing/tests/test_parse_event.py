from sfy import event
from datetime import datetime, timezone
from . import sfyhub


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

def test_position_range(sfyhub, tmpdir):
    b = sfyhub.buoy('bug08')
    pcks = b.position_packages_range(
        datetime(2022, 8, 14, 00, 00, tzinfo=timezone.utc),
        datetime(2022, 8, 15, 23, 59, tzinfo=timezone.utc))
    print(pcks)

