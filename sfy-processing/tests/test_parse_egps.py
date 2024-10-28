import pytest
import numpy as np
from datetime import datetime, timezone
from sfy.egps import Egps, EgpsCollection
from . import *


def test_parse_table():
    d = open(
        'tests/data/rtk01/v2_egps.qo.json'
    ).read()
    e = Egps.parse(d)
    print(e)

    assert len(e.n) == 124
    assert len(e.e) == 124
    assert len(e.z) == 124

    assert len(e.vz) == 124
    assert len(e.ve) == 124
    assert len(e.vn) == 124

@needs_hub
def test_collect(sfyhub):
    b = sfyhub.buoy("dev864593051335148")
    pcks = b.egps_packages_range(
        datetime(2024, 10, 28, 12, 40, tzinfo=timezone.utc),
        datetime(2024, 10, 28, 13, 20, tzinfo=timezone.utc))
    assert len(pcks) > 2

    c = EgpsCollection(pcks)
    print("duration:", c.duration)
    print(f"len= {len(pcks)}")
    print(c)

    assert c.start == c.time[0]
