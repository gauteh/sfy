import numpy as np
from sfy import axl


def test_parse_table():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    print(a)

    assert len(a.x) == 1024
    assert len(a.y) == 1024
    assert len(a.z) == 1024


def test_synthetic_linear():
    d = open(
        'tests/data/dev864475044203262/1639731747990-8c7f35f8-176f-4ae2-8faa-83ea347a345f_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    print(a)

    s = np.arange(0, 3072).astype(np.float16)
    sx = s[0::3]
    sy = s[1::3]
    sz = s[2::3]

    assert len(a.x) == 1024
    assert len(a.y) == 1024
    assert len(a.z) == 1024

    np.testing.assert_array_equal(sx, a.x)
    np.testing.assert_array_equal(sy, a.y)
    np.testing.assert_array_equal(sz, a.z)

def test_parse_save_parse(tmpdir):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    pth = tmpdir / "test.json"
    a.save(pth)

    a2 = axl.Axl.from_file(pth)
    assert a == a2

def test_parse_json_parse(tmpdir):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    a2 = axl.Axl.parse(a.json())

    assert a == a2

def test_parse_rt_post(tmpdir):
    d = open(
        'tests/data/cont/rt01.json'
    ).read()
    a = axl.Axl.parse(d)
    print(a)
    print(a.z)

