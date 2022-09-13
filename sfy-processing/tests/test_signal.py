import numpy as np
import scipy as sc
from sfy import axl, signal
import matplotlib.pyplot as plt


def test_velocity():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    x, y, z = signal.velocity(a)
    assert len(x) == (len(a.x)-1)

def test_displacement():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    x, y, z = signal.displacement(a)
    assert len(x) == (len(a.x)-2)

def test_integration_dft(plot):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    z = a.z

    z = z - np.mean(z)
    z = sc.signal.detrend(z)

    zz = signal.integrate_dft(z, a.frequency)

    assert len(zz) == len(z)

    if plot:
        plt.figure()
        plt.plot(a.time, z)
        plt.plot(a.time, zz)
        plt.show()
