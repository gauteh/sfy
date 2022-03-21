import numpy as np
from sfy import axl, signal


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

