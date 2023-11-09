import numpy as np
import scipy as sc
from datetime import datetime, timezone
from sfy import axl, signal
from pytest import approx
import matplotlib.pyplot as plt

from . import *


@needs_hub
def test_axl_v5_quiet(sfyhub, plot):
    b = sfyhub.buoy("dev867648043595907")
    pcks = b.axl_packages_range(
        datetime(2022, 12, 11, 15, 00, tzinfo=timezone.utc),
        datetime(2022, 12, 11, 15, 30, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()

    assert len(ds.time) / ds.frequency > (20 * 60)

    # this is a period where the buoy was resting quietly
    print(np.mean(ds.w_z))
    print(np.std(ds.w_z))
    print(np.max(np.abs(np.mean(ds.w_z) - ds.w_z)))
    assert np.mean(ds.w_z) == approx(9.8, abs=0.2)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.33)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

    if plot:
        f, P = sc.signal.welch(ds.w_z, fs=ds.frequency, nperseg=4096)
        plt.figure()
        plt.loglog(f, P)
        plt.show()


@needs_hub
def test_axl_v3_quiet(sfyhub):
    b = sfyhub.buoy("dev867648043598489")
    pcks = b.axl_packages_range(
        datetime(2022, 11, 30, 11, 0, tzinfo=timezone.utc),
        datetime(2022, 11, 30, 11, 25, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)


@needs_hub
def test_axl_v6_16g_range(sfyhub, plot):
    # this is a desktop test with buoy configured to 16g, 1000 dps after fixing gyro-accel driver scale error.
    import base64

    axl.Axl.__keep_payload__ = True
    b = sfyhub.buoy('bug29')
    pcks = b.axl_packages_range(
        datetime(2023, 11, 6, 9, 40, tzinfo=timezone.utc),
        datetime(2023, 11, 6, 11, 50, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    c = c.clip(datetime(2023, 11, 6, 10, 40, tzinfo=timezone.utc),
               datetime(2023, 11, 6, 10, 50, tzinfo=timezone.utc))

    axl.Axl.__keep_payload__ = False

    assert c.pcks[0].accel_range == 16
    assert c.pcks[0].gyro_range == 1000

    payload = c.pcks[0].payload
    payload = base64.b64decode(payload)
    payload = np.frombuffer(payload, dtype=np.uint16)

    z = axl.scale_u16_to_f32(2 * 16 * axl.SENSORS_GRAVITY_STANDARD,
                             payload[2::3])
    MAX = 16 * axl.SENSORS_GRAVITY_STANDARD
    print(np.max(z), MAX)
    assert np.max(z) <= (MAX + .2)

    print(np.median(z) / axl.SENSORS_GRAVITY_STANDARD)
    ds = c.to_dataset()
    print(ds)

    np.testing.assert_array_equal(z + axl.SENSORS_GRAVITY_STANDARD,
                                  ds.w_z[:1024])

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

    if plot:
        import matplotlib.pyplot as plt
        (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

        ww_z = signal.bandpass(ds.w_z, 1 / 52)
        plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)
        plt.show()

@needs_hub
def test_axl_v6_4g_range(sfyhub, plot):
    # this is a desktop test with buoy configured to 4g, 500 dps after fixing gyro-accel driver scale error.
    import base64

    axl.Axl.__keep_payload__ = True
    b = sfyhub.buoy('bug29')
    pcks = b.axl_packages_range(
        datetime(2023, 11, 6, 9, 40, tzinfo=timezone.utc),
        datetime(2023, 11, 6, 12, 50, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    c = c.clip(datetime(2023, 11, 6, 11, 45, tzinfo=timezone.utc),
               datetime(2023, 11, 6, 12, 00, tzinfo=timezone.utc))

    axl.Axl.__keep_payload__ = False

    assert c.pcks[0].accel_range == 4
    assert c.pcks[0].gyro_range == 500

    payload = c.pcks[0].payload
    payload = base64.b64decode(payload)
    payload = np.frombuffer(payload, dtype=np.uint16)

    z = axl.scale_u16_to_f32(2 * 4 * axl.SENSORS_GRAVITY_STANDARD,
                             payload[2::3])
    MAX = 16 * axl.SENSORS_GRAVITY_STANDARD
    print(np.max(z), MAX)
    assert np.max(z) <= (MAX + .2)

    print(np.median(z) / axl.SENSORS_GRAVITY_STANDARD)
    ds = c.to_dataset()
    print(ds)

    np.testing.assert_array_equal(z + axl.SENSORS_GRAVITY_STANDARD,
                                  ds.w_z[:1024])

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

    if plot:
        import matplotlib.pyplot as plt
        (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

        ww_z = signal.bandpass(ds.w_z, 1 / 52)
        plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)
        plt.show()

@needs_hub
def test_v6_16g_range_bali(sfyhub, plot):
    # this is a desktop test with buoy configured to 16g, 1000 dps after fixing gyro-accel driver scale error.
    import base64

    axl.Axl.__keep_payload__ = True
    b = sfyhub.buoy('bug32')
    pcks = b.axl_packages_range(
        datetime(2023, 11, 9, 15, 00, tzinfo=timezone.utc),
        datetime(2023, 11, 9, 16, 50, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    c = c.clip(datetime(2023, 11, 9, 15, 35, 40, tzinfo=timezone.utc),
               datetime(2023, 11, 9, 15, 57, 5, tzinfo=timezone.utc))

    axl.Axl.__keep_payload__ = False

    assert c.pcks[0].accel_range == 16
    assert c.pcks[0].gyro_range == 1000

    payload = c.pcks[0].payload
    payload = base64.b64decode(payload)
    payload = np.frombuffer(payload, dtype=np.uint16)

    z = axl.scale_u16_to_f32(2 * 16 * axl.SENSORS_GRAVITY_STANDARD,
                             payload[2::3])
    MAX = 16 * axl.SENSORS_GRAVITY_STANDARD
    print(np.max(z), MAX)
    assert np.max(z) <= (MAX + .2)

    print(np.median(z) / axl.SENSORS_GRAVITY_STANDARD)
    ds = c.to_dataset()
    print(ds)

    np.testing.assert_array_equal(z + axl.SENSORS_GRAVITY_STANDARD,
                                  ds.w_z[:1024])

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

    if plot:
        import matplotlib.pyplot as plt
        (ds.w_z / axl.SENSORS_GRAVITY_STANDARD).plot()

        ww_z = signal.bandpass(ds.w_z, 1 / 52)
        plt.plot(ds.time, ww_z / axl.SENSORS_GRAVITY_STANDARD)
        plt.show()
