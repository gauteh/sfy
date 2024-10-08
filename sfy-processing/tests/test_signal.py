import numpy as np
import scipy as sc
from sfy import axl, signal
import sfy
import matplotlib.pyplot as plt
from sfy.axl import AxlCollection
from datetime import datetime, timezone
from . import *


def test_integration_dft(plot):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    z = a.z

    z = z - np.mean(z)
    z = sc.signal.detrend(z)

    zz = signal.dft_integrate(z, a.frequency)
    zc = sc.integrate.cumulative_trapezoid(z, dx=a.dt)

    assert len(zz) == len(z)

    if plot:
        plt.figure()
        plt.plot(a.time, z, label='accel')
        plt.plot(a.time, zz, label='integrate (dft)')
        plt.plot(a.time[1:], zc, label='integrate (cumtrapz)')
        plt.legend()
        plt.show()

    np.testing.assert_array_almost_equal(zz[1:], zc, decimal=5)


def test_adjust_fir():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    x = a.to_dataset()
    print(x)
    x1 = signal.adjust_fir_filter(x, False)
    print(x1)

    assert all(x1['time'] == x['time'])


@needs_hub
def test_pca_xy(sfyhub, plot):
    b = sfyhub.buoy("wavebug25")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 17, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 19, tzinfo=timezone.utc))
    c = AxlCollection(pcks)
    ds = c.to_dataset()

    x, y, xv, yv, u0, u1 = signal.reproject_pca(ds.w_x, ds.w_y)
    print(xv, yv)

    assert xv > yv
    assert np.var(x) > np.var(ds.w_x)
    assert np.var(y) < np.var(ds.w_y)
    np.testing.assert_array_almost_equal(np.sqrt(x**2 + y**2),
                                         np.sqrt(ds.w_x**2 + ds.w_y**2))

    u = sfy.xr.displacement(ds, filter_freqs=[0.15, 25])
    xd, yd, xvd, yvd, ud0, ud1 = signal.reproject_pca(u.u_x, u.u_y)
    print(u0, u1)
    print(ud0, ud1)

    if plot:
        plt.figure()
        plt.plot(u.time, u.u_z)
        plt.show()
