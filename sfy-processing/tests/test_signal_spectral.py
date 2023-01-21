import numpy as np
import scipy as sc
from sfy import axl, signal
import matplotlib.pyplot as plt

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
