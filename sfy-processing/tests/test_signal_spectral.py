import numpy as np
import scipy as sc
from sfy import axl, signal
from sfy.axl import AxlCollection
import matplotlib.pyplot as plt
from datetime import datetime, timezone
from . import *


def test_calc_hs():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    z = signal.integrate(a.z, a.dt, order=2, filter=False)
    hs = signal.hs(z)
    assert hs < 0.01
    print(hs)


def test_welch(plot):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    f, P = signal.welch(a.frequency, a.z)

    if plot:
        plt.figure()
        plt.loglog(f, P, label='welch accel')
        plt.legend()
        plt.show()


def test_calc_hm0():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    f, P = signal.welch(a.frequency, a.z)
    hm0 = signal.hm0(f, P)
    assert hm0 < 0.01
    print(hm0)


def test_calc_hm0_timeseries():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    hm0 = a.hm0()
    print(hm0)
    assert hm0 < 0.01

    f, P = signal.welch(a.frequency, a.z, order=2)
    hm0d = signal.hm0(f, P)
    print(hm0d)
    assert hm0 == hm0d

@needs_hub
def test_hm0_collection(sfyhub):
    b = sfyhub.buoy("dev864475044204278")
    pcks = b.axl_packages_range(
        datetime(2022, 4, 26, 11, 34, tzinfo=timezone.utc),
        datetime(2022, 4, 26, 12, 35, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    hm0 = c.hm0()
    print(hm0)

@needs_hub
def test_imu_cutoff_rabault2022(sfyhub, plot):
    b = sfyhub.buoy("wavebug26")
    pcks = b.axl_packages_range(
        datetime(2023, 1, 23, 5, 34, tzinfo=timezone.utc),
        datetime(2023, 1, 23, 6, 35, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    f, P = signal.welch(c.frequency, c.z)
    ci, cf, EP = signal.imu_cutoff_rabault2022(f, P)

    print(ci, cf, P[ci])
    if plot:
        plt.figure()
        plt.plot(f, P)
        plt.plot(f, EP)
        plt.plot(cf, P[ci], 'x')
        plt.show()
