import os
import pytest

from sfy import hub

@pytest.fixture
def sfy():
    from urllib.parse import urljoin

    API = os.getenv('SFY_SERVER')
    KEY = os.getenv('SFY_READ_TOKEN')

    if API is None or KEY is None:
        raise Exception("No API and KEY")

    API = urljoin(API, 'buoys')
    return hub.Hub(urljoin(API, 'buoys'), KEY)


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

