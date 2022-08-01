import pytest
from datetime import datetime, timezone, timedelta

from sfy import hub
from . import sfyhub


def test_list_buoys(sfyhub):
    print(sfyhub.buoys())


def test_get_buoy(sfyhub):
    b = sfyhub.buoy("867730051260788")
    assert b.dev == "dev867730051260788"


def test_list_packages(sfyhub):
    b = sfyhub.buoy("867730051260788")
    print(b.packages())


def test_get_raw_package(sfyhub):
    b = sfyhub.buoy("dev864475044204278")
    pck = b.raw_package(
        '1650973616744-42e2549d-868b-4c46-a7ef-723c7a1e6418_axl.qo.json')


def test_get_package(sfyhub):
    b = sfyhub.buoy("dev864475044204278")
    pck = b.package(
        '1650973616744-42e2549d-868b-4c46-a7ef-723c7a1e6418_axl.qo.json')
    print(pck)


def test_get_last(sfyhub, benchmark):
    b = sfyhub.buoy("867730051260788")
    pck = benchmark(b.last)
    print(pck)


def test_list_packages_range(sfyhub):
    b = sfyhub.buoy("867730051260788")
    start = datetime(2022, 1, 21, tzinfo=timezone.utc)
    pcks = b.packages_range(start=start)
    assert all((pck[0] > start for pck in pcks))


def test_fetch_raw_range(sfyhub):
    b = sfyhub.buoy("867730051260788")
    start = datetime(2022, 1, 21, tzinfo=timezone.utc)
    pcks = b.packages_range(start=start)
    print(pcks)
    print(len(pcks))
    assert all((pck[0] > start for pck in pcks))

def test_fetch_packages_range(sfyhub):
    b = sfyhub.buoy("867730051260788")
    start = datetime(2022, 3, 29, tzinfo=timezone.utc)
    end = datetime(2022, 3, 29, 1, tzinfo=timezone.utc)
    pcks = b.fetch_packages_range(start=start, end=end)
    print(pcks)
    print(len(pcks))

def test_fetch_packages_range_caching(sfyhub):
    b = sfyhub.buoy("867730051260788")
    start = datetime(2022, 3, 29, tzinfo=timezone.utc)
    end = datetime(2022, 3, 29, 1, tzinfo=timezone.utc)
    fpcks = b.fetch_packages_range(start=start, end=end)

    pcks = b.packages_range(start, end)

    assert len(pcks) == len(fpcks)
    assert len(pcks) == 211

    assert all((a[1] == b[1] for (a, b) in zip(fpcks, pcks)))

    # these should be partially uncached
    start = datetime(2022, 3, 29, tzinfo=timezone.utc)
    end = datetime(2022, 3, 29, 4, tzinfo=timezone.utc)
    pcks = b.packages_range(start=start, end=end)
    assert len(pcks) == 726

    fpcks = b.fetch_packages_range(start, end)
    assert len(fpcks) == len(pcks)
    assert all((a[1] == b[1] for (a, b) in zip(fpcks, pcks)))

    # create caching gap
    start = datetime(2022, 3, 29, 6, tzinfo=timezone.utc)
    end = datetime(2022, 3, 29, 7, tzinfo=timezone.utc)
    pcks = b.packages_range(start=start, end=end)
    assert len(pcks) == 132
    fpcks = b.fetch_packages_range(start, end)
    assert len(fpcks) == len(pcks)
    assert all((a[1] == b[1] for (a, b) in zip(fpcks, pcks)))

    # get everything, including gap
    start = datetime(2022, 3, 29, tzinfo=timezone.utc)
    end = datetime(2022, 3, 29, 7, tzinfo=timezone.utc)
    pcks = b.packages_range(start=start, end=end)
    assert len(pcks) == 1244
    fpcks = b.fetch_packages_range(start, end)
    assert len(fpcks) == len(pcks)
    assert all((a[1] == b[1] for (a, b) in zip(fpcks, pcks)))
