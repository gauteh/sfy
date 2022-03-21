import os
import pytest
from datetime import datetime, date, time, timezone

from sfy import hub

@pytest.fixture
def sfy(tmpdir):
    h = hub.Hub.from_env()
    h.cache = tmpdir
    return h

def test_list_buoys(sfy):
    print(sfy.buoys())

def test_get_buoy(sfy):
    b = sfy.buoy("867730051260788")
    assert b.dev == "dev867730051260788"

def test_list_packages(sfy):
    b = sfy.buoy("867730051260788")
    print(b.packages())

def test_get_raw_package(sfy):
    b = sfy.buoy("867730051260788")
    pck = b.raw_package('1647857681694-a90b61ed-4244-4785-a797-413c411d636c_axl.qo.json')

def test_get_package(sfy):
    b = sfy.buoy("867730051260788")
    pck = b.package('1647857681694-a90b61ed-4244-4785-a797-413c411d636c_axl.qo.json')
    print(pck)

def test_list_packages_range(sfy):
    b = sfy.buoy("867730051260788")
    start = datetime(2022,1, 21, tzinfo = timezone.utc)
    pcks = b.packages_range(start=start)
    assert all((pck[0] > start for pck in pcks))
