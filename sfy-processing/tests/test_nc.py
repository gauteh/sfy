import numpy as np
from sfy import axl, signal
from sfy.axl import AxlCollection
from . import *
from datetime import datetime, timezone


def test_pck_nc(tmpdir):
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)
    ds = a.to_dataset()
    print(ds)
    a.to_netcdf(tmpdir / "test.nc")


@needs_hub
def test_collection_nc(sfyhub, tmpdir):
    b = sfyhub.buoy("dev864475044204278")
    pcks = b.axl_packages_range(
        datetime(2022, 4, 26, 11, 34, tzinfo=timezone.utc),
        datetime(2022, 4, 26, 11, 35, tzinfo=timezone.utc))
    c = AxlCollection(pcks)
    ds = c.to_dataset()
    print(ds)
    c.to_netcdf(tmpdir / "test.nc")

@needs_hub
def test_buggy_data(sfyhub, tmpdir):
    # sfydata axl ts --start 2022-07-25 --end 2022-08-15 bug08 --file bug08UnstadJuly.nc
    b = sfyhub.buoy('bug08')
    pcks = b.axl_packages_range(
        datetime(2022, 8, 14, 00, 00, tzinfo=timezone.utc),
        datetime(2022, 8, 15, 23, 59, tzinfo=timezone.utc))
    print(pcks)
    c = AxlCollection(pcks)
    ds = c.to_dataset()
    print(ds)

