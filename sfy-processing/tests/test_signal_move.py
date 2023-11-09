import numpy as np
import scipy as sc
from datetime import datetime, timezone
from sfy import axl, signal, xr as sxr
from pytest import approx
import matplotlib.pyplot as plt

from . import *


@needs_hub
def test_v6_16g_range_bali_1m(sfyhub, plot):
    # this as test with buoy configured to 16g, 1000 dps after fixing gyro-accel driver scale error.
    # the buoy was moved up and down from a table, around less than 1m

    axl.Axl.__keep_payload__ = True
    b = sfyhub.buoy('bug32')
    pcks = b.axl_packages_range(
        datetime(2023, 11, 9, 15, 00, tzinfo=timezone.utc),
        datetime(2023, 11, 9, 16, 50, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    c = c.clip(datetime(2023, 11, 9, 15, 31, 55, tzinfo=timezone.utc),
               datetime(2023, 11, 9, 15, 33, 36, tzinfo=timezone.utc))

    axl.Axl.__keep_payload__ = False

    assert c.pcks[0].accel_range == 16
    assert c.pcks[0].gyro_range == 1000

    ds = c.to_dataset()
    print(ds)

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

    u = sxr.displacement(ds)

    if plot:
        import matplotlib.pyplot as plt

        (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

        ww_z = signal.bandpass(ds.w_z, 1 / 52)
        plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)

        plt.figure()
        plt.plot(u.time, u.u_z)

        plt.show()

    diff = u.u_z.max() - u.u_z.min()
    assert diff > 0.6 and diff < 1.0

@needs_hub
def test_v6_16g_range_bali_161cm(sfyhub, plot):
    # this as test with buoy configured to 16g, 1000 dps after fixing gyro-accel driver scale error.
    # the buoy was moved up and down from the ground to above 161 cm

    axl.Axl.__keep_payload__ = True
    b = sfyhub.buoy('bug32')
    pcks = b.axl_packages_range(
        datetime(2023, 11, 9, 15, 00, tzinfo=timezone.utc),
        datetime(2023, 11, 9, 16, 50, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    # c = c.clip(datetime(2023, 11, 9, 15, 24, 50, tzinfo=timezone.utc),
    #            datetime(2023, 11, 9, 15, 26, 13, tzinfo=timezone.utc))

    axl.Axl.__keep_payload__ = False

    assert c.pcks[0].accel_range == 16
    assert c.pcks[0].gyro_range == 1000

    ds = c.to_dataset()
    print(ds)

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

    u = sxr.displacement(ds, filter_freqs=[0.02, 25])
    u = u.sel(time=slice('2023-11-09T15:25:36', '2023-11-09T15:26:13'))
    u['u_z'] = u['u_z'] - u['u_z'].mean()

    if plot:
        import matplotlib.pyplot as plt

        (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

        ww_z = signal.bandpass(ds.w_z, 1 / 52)
        plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)

        plt.figure()
        plt.plot(u.time, u.u_z)

        plt.show()

    diff = u.u_z.max() - u.u_z.min()
    print(diff)
    assert diff > 1.6 and diff < 1.8

def test_direction_integrate(plot):
    fs = 52
    dt = 1/fs
    t = np.arange(0, 200, dt)
    win = 0.2 * np.hamming(3 * fs) + 9.81
    print(win.shape)
    print(t.shape)

    s = np.full(t.shape, 9.81)
    s[4000:(4000+len(win))] = win

    # A positive acceleration means that the buoy is lifted up. It feels a greater acceleration compared to just sitting still.
    u_z = -signal.integrate(s, dt, order=2, freqs=[0.1, 25], method='dft')
    u_z_t = -signal.integrate(s, dt, order=2, freqs=[0.1, 25], method='trapz')

    assert np.max(u_z) > np.abs(np.min(u_z)), "the positive peak should be greatest for a positive movement"

    if plot:
        plt.figure()
        plt.plot(t, s)
        plt.plot(t, u_z)
        plt.plot(t[:-1], u_z_t, '--')
        plt.show()

