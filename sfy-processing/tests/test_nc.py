import numpy as np
from sfy import axl, signal
from sfy.axl import AxlCollection
from . import sfyhub
from datetime import datetime, timezone


def test_pck_nc(tmpdir):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    ds = a.to_dataset()
    print(ds)
    a.to_netcdf(tmpdir / "test.nc")


def test_collection_nc(sfyhub, tmpdir):
    b = sfyhub.buoy("dev864475044204278")
    pcks = b.axl_packages_range(
        datetime(2022, 4, 26, 11, 34, tzinfo=timezone.utc),
        datetime(2022, 4, 26, 11, 35, tzinfo=timezone.utc))
    c = AxlCollection(pcks)
    ds = c.to_dataset()
    print(ds)
    c.to_netcdf(tmpdir / "test.nc")
