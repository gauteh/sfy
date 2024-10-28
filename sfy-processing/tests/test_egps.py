import pytest
import numpy as np
from datetime import datetime, timezone
from sfy.egps import Egps, EgpsCollection
import matplotlib.pyplot as plt
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

@needs_hub
def test_stationary(sfyhub, plot):
    b = sfyhub.buoy('dev864593051335148')
    pcks = b.egps_packages_range(
        datetime(2024, 10, 28, 12, 40, tzinfo=timezone.utc),
        datetime(2024, 10, 28, 13, 20, tzinfo=timezone.utc))

    c = EgpsCollection(pcks)
    ds = c.to_dataset()
    print(ds)

    ds = ds.sel(time=slice('2024-10-28T12:40', '2024-10-28T13:00'))

    if plot:
        plt.figure()
        z = ds.z / 1000.
        z.plot()
        plt.grid()
        plt.show()

    print(ds.z.mean())

    # assert ds.w_z.mean() == approx(9.81, abs=0.3)
