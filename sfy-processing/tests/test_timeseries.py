import numpy as np
import pandas as pd
from datetime import datetime, timezone
from pytest import approx
import matplotlib.pyplot as plt
import xarray as xr

import sfy.xr
from sfy.axl import AxlCollection
from . import *

@needs_hub
def test_time(sfyhub):
    b = sfyhub.buoy("dev867648043576717")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 8, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 38, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    ds = c.to_dataset()

    p0 = c.pcks[0]

    assert p0.offset == 0
    assert p0.start.timestamp() == p0.timestamp / 1000.

    assert p0.end.timestamp() == approx((p0.start + pd.Timedelta(p0.duration, 's')).timestamp())

@needs_hub
def test_estimate_frequency(sfyhub):
    b = sfyhub.buoy("dev867648043576717")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 8, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 38, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    ds = c.to_dataset()

    f = sfy.xr.estimate_frequency(ds)
    print(f)
    assert len(f) == 63
    assert len(f) == len(c)

    assert np.all(f-52 < .1 * 52)

@needs_hub
def test_retime(sfyhub, plot):
    b = sfyhub.buoy("dev867648043576717")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 8, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 38, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    ds = c.to_dataset()

    ds2 = sfy.xr.retime(ds)
    print(ds2)

    assert not np.all(ds2.time.values == ds.time.values)
    assert len(np.unique(ds2.time)) == len(ds2.time)

    np.testing.assert_array_equal(ds2.oldtime, ds.time)


    assert np.max(ds2.time) > ds.time[0]
    assert np.min(ds2.time) < ds.time[-1]

    if plot:
        plt.figure()
        ds.w_z.plot()
        ds2.w_z.plot()

        plt.show()

    ds2 = ds2.sel(time=slice('2023-04-20 09:09:00', '2023-04-20 09:11:00'))
    print(ds2)

@needs_hub
def test_retime_sintef(sfyhub, plot):
    b = sfyhub.buoy("dev867648043599644")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 16, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 40, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    ds = c.to_dataset()

    ds2 = sfy.xr.retime(ds)

    assert not np.all(ds2.time.values == ds.time.values)
    assert len(np.unique(ds2.time)) == len(ds2.time)

    np.testing.assert_array_equal(ds2.oldtime, ds.time)

    t0 = pd.Timestamp('2023-04-20 09:16:00')
    t1 = pd.Timestamp('2023-04-20 09:20:00')

    print(ds.time.values[[0, -1]])
    ds = ds.sel(time=slice(t0, t1))

    print(ds.time.values[[0, -1]])
    print(ds2.time.values[[0, -1]])

    if plot:
        plt.figure()
        ds.w_z.plot()
        ds2.w_z.plot()

        plt.show()

@needs_hub
def test_retime_group(sfyhub):
    b = sfyhub.buoy("dev867648043599644")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 16, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 40, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    ds = c.to_dataset()

    s = sfy.xr.groupby_segments(ds)
    print(s)

    # This dataset has no gaps
    ds2 = sfy.xr.groupby_segments(ds).map(lambda d: d)
    print(ds2)

    assert ds == ds2

