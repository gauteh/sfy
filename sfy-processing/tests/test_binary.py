"""
Tests for binary package fetching (axlb.qo / egpsb.qo).

Uses a device with only binary packages captured 2026-04-11 ~15:08-15:30 UTC.
"""
import pytest
import numpy as np
from datetime import datetime, timezone
from pytest import approx

from sfy.axl import Axl, AxlCollection
from sfy.egps import Egps, EgpsCollection
from . import *

DEV = "860264051909217"
TX_START = datetime(2026, 4, 11, 15, 5, tzinfo=timezone.utc)
TX_END   = datetime(2026, 4, 11, 15, 35, tzinfo=timezone.utc)


@needs_hub
def test_axlb_packages_range(sfyhub):
    b = sfyhub.buoy(DEV)
    pcks = b.axl_packages_range(TX_START, TX_END, binary=True)
    assert len(pcks) > 50
    assert all(isinstance(p, Axl) for p in pcks)


@needs_hub
def test_egpsb_packages_range(sfyhub):
    b = sfyhub.buoy(DEV)
    pcks = b.egps_packages_range(TX_START, TX_END, binary=True)
    assert len(pcks) > 50
    assert all(isinstance(p, Egps) for p in pcks)


@needs_hub
def test_axlb_only_binary(sfyhub):
    """Device only has binary packages; non-binary query returns empty."""
    b = sfyhub.buoy(DEV)
    pcks = b.axl_packages_range(TX_START, TX_END, binary=False)
    assert len(pcks) == 0


@needs_hub
def test_egpsb_only_binary(sfyhub):
    """Device only has binary packages; non-binary query returns empty."""
    b = sfyhub.buoy(DEV)
    pcks = b.egps_packages_range(TX_START, TX_END, binary=False)
    assert len(pcks) == 0


@needs_hub
def test_axlb_collection(sfyhub):
    b = sfyhub.buoy(DEV)
    pcks = b.axl_packages_range(TX_START, TX_END, binary=True)
    c = AxlCollection(pcks)
    assert c.frequency == 52
    assert c.duration > 800.0
    assert len(c) > 50


@needs_hub
def test_axlb_stationary(sfyhub):
    """Stationary device on desk: z-axis acceleration should be ~1g (9.81 m/s²)."""
    b = sfyhub.buoy(DEV)
    pcks = b.axl_packages_range(TX_START, TX_END, binary=True)
    z = np.concatenate([p.z for p in pcks])
    assert float(np.mean(z)) == approx(9.81, abs=0.3)
    # horizontal axes should be near zero
    x = np.concatenate([p.x for p in pcks])
    y = np.concatenate([p.y for p in pcks])
    assert float(np.abs(np.mean(x))) < 0.5
    assert float(np.abs(np.mean(y))) < 0.5


@needs_hub
def test_axlb_location(sfyhub):
    b = sfyhub.buoy(DEV)
    pcks = b.axl_packages_range(TX_START, TX_END, binary=True)
    assert pcks[0].lat == approx(60.33, abs=0.1)
    assert pcks[0].lon == approx(5.37, abs=0.1)


@needs_hub
def test_egpsb_collection(sfyhub):
    b = sfyhub.buoy(DEV)
    pcks = b.egps_packages_range(TX_START, TX_END, binary=True)
    c = EgpsCollection(pcks)
    assert c.frequency == approx(14.08, abs=0.1)
    assert c.duration > 800.0
    assert len(c) > 50


@needs_hub
def test_egpsb_stationary(sfyhub):
    """Stationary device: velocities should be near zero (GPS noise level)."""
    b = sfyhub.buoy(DEV)
    pcks = b.egps_packages_range(TX_START, TX_END, binary=True)
    c = EgpsCollection(pcks)
    ds = c.to_dataset()
    assert float(np.abs(ds.vz).mean()) < 100.0   # mm/s
    assert float(np.abs(ds.vn).mean()) < 100.0
    assert float(np.abs(ds.ve).mean()) < 100.0


@needs_hub
def test_egpsb_elevation(sfyhub):
    """Elevation (z in mm) should be in a reasonable range for the test location."""
    b = sfyhub.buoy(DEV)
    pcks = b.egps_packages_range(TX_START, TX_END, binary=True)
    c = EgpsCollection(pcks)
    ds = c.to_dataset()
    z_mean_m = float(ds.z.mean()) / 1000.0
    assert 50.0 < z_mean_m < 200.0


@needs_hub
def test_egpsb_location(sfyhub):
    """lat/lon in egps payload are in units of 1e-7 degrees."""
    b = sfyhub.buoy(DEV)
    pcks = b.egps_packages_range(TX_START, TX_END, binary=True)
    lat_deg = pcks[0].lat / 1e7
    lon_deg = pcks[0].lon / 1e7
    assert lat_deg == approx(60.33, abs=0.1)
    assert lon_deg == approx(5.37, abs=0.1)
