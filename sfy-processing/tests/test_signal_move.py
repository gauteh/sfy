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

# @needs_hub
# def test_v6_16g_range_bali_hip_head(sfyhub, plot):
#     # this as test with buoy configured to 16g, 1000 dps after fixing gyro-accel driver scale error.
#     # the buoy was moved up and down from the ground to above 161 cm

#     b = sfyhub.buoy('bug32')
#     pcks = b.axl_packages_range(
#         datetime(2023, 11, 10, 11 - 8, tzinfo=timezone.utc),
#         datetime(2023, 11, 10, 13 - 8, tzinfo=timezone.utc))

#     c = axl.AxlCollection(pcks)
#     ds = c.to_dataset()
#     print(ds)

#     u = sxr.displacement(ds, filter_freqs=[0.05, 25])
#     u = u.sel(time=slice('2023-11-10T03:36:19', '2023-11-10T03:43:12'))
#     u['u_z'] = u['u_z'] - u['u_z'].mean()

#     ds = ds.sel(time=slice('2023-11-10T03:36:19', '2023-11-10T03:43:12'))

#     if plot:
#         import matplotlib.pyplot as plt

#         (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

#         ww_z = signal.bandpass(ds.w_z, 1 / 52)
#         plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)

#         plt.figure()
#         plt.plot(u.time, u.u_z)

#         plt.show()

#     diff = u.u_z.max() - u.u_z.min()
#     print(diff)
#     assert diff > 1.6 and diff < 1.8

def test_direction_integrate(plot):
    fs = 52
    dt = 1/fs
    t = np.arange(0, 200, dt)
    win = 0.2 * np.hamming(10 * fs) + 9.81
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

@needs_hub
def test_v6_16g_range_window_20cm_1m_test(sfyhub, plot):
    # 12:43 LT: window
    # 12:47 LT: three taps + 3x lift 20 cm + rest + 3 taps + 3 x lift 1m + rest
    # 12:50 LT: ^^ done,
    # 12:53 LT: reset to initiate sync.

    b = sfyhub.buoy('bug30')
    pcks = b.axl_packages_range(
        datetime(2023, 11, 10, 11, tzinfo=timezone.utc),
        datetime(2023, 11, 10, 13, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()
    print(ds)

    u = sxr.displacement(ds, filter_freqs=[0.1, 25])
    u = u.sel(time=slice('2023-11-10T11:47:00', '2023-11-10T11:51:00'))
    u['u_z'] = u['u_z'] - u['u_z'].mean()

    ds = ds.sel(time=slice('2023-11-10T11:47:00', '2023-11-10T11:51:00'))

    # 3x ca 20 cm

    uu = u.sel(time=slice('2023-11-10T11:49:15', '2023-11-10T11:49:45'))
    dds = ds.sel(time=slice('2023-11-10T11:49:15', '2023-11-10T11:49:45'))

    diff = uu.u_z.max() - uu.u_z.min()
    print(diff)
    assert diff > 0.15 and diff < 0.35

    # 3x ca 1m
    u = sxr.displacement(ds, filter_freqs=[0.05, 25])
    u['u_z'] = u['u_z'] - u['u_z'].mean()
    uu = u.sel(time=slice('2023-11-10T11:50:02', '2023-11-10T11:50:53'))
    dds = ds.sel(time=slice('2023-11-10T11:50:02', '2023-11-10T11:50:53'))

    diff = uu.u_z.max() - uu.u_z.min()
    print(diff)
    assert diff > 0.99 and diff < 1.1

    if plot:
        import matplotlib.pyplot as plt

        (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

        ww_z = signal.bandpass(ds.w_z, 1 / 52)
        plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)

        plt.figure()
        plt.plot(uu.time, uu.u_z)

        plt.show()

@needs_hub
def test_at_rest(sfyhub, plot):
    b = sfyhub.buoy('dev867648043600996')
    pcks = b.axl_packages_range(
        datetime(2024, 2, 19, 9, 0, tzinfo=timezone.utc),
        datetime(2024, 2, 19, 10, 11, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()
    print(ds)

    ds = ds.sel(time=slice('2024-02-19T9:05', '2024-02-19T9:21'))

    print(ds.w_z.mean())


    if plot:
        plt.figure()
        ds.w_z.plot()
        plt.show()

    assert ds.w_z.mean() == approx(9.81, abs=0.3)

@needs_hub
def test_lift(sfyhub, plot):
    b = sfyhub.buoy('dev867648043600996')
    pcks = b.axl_packages_range(
        datetime(2024, 2, 19, 9, 5, tzinfo=timezone.utc),
        datetime(2024, 2, 19, 10, 11, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()
    print(ds)

    # time 9:42 UTC
    ds = ds.sel(time=slice('2024-02-19T9:39', '2024-02-19T9:45'))

    u = sxr.displacement(ds)

    if plot:
        plt.figure()
        ds.w_z.plot()
        u.u_z.plot()
        plt.show()

    print(ds.w_z.mean())

    assert ds.w_z.mean() == approx(9.81, abs=0.3)
