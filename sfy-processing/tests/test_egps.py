import pytest
import numpy as np
from datetime import datetime, timezone
from sfy.egps import Egps, EgpsCollection
import matplotlib.pyplot as plt
from . import *


def test_parse_table():
    d = open(
        'tests/data/rtk01/v2_egps.qo.json'
    ).read()
    e = Egps.parse(d)
    print(e)

    assert len(e.n) == 124
    assert len(e.e) == 124
    assert len(e.z) == 124

    assert len(e.vz) == 124
    assert len(e.ve) == 124
    assert len(e.vn) == 124

@needs_hub
def test_collect(sfyhub):
    b = sfyhub.buoy("dev864593051335148")
    pcks = b.egps_packages_range(
        datetime(2024, 10, 28, 12, 40, tzinfo=timezone.utc),
        datetime(2024, 10, 28, 13, 20, tzinfo=timezone.utc))
    assert len(pcks) > 2

    c = EgpsCollection(pcks)
    print("duration:", c.duration)
    print(f"len= {len(pcks)}")
    print(c)

    assert c.start == c.time[0]

@needs_hub
def test_stationary(sfyhub, plot):
    b = sfyhub.buoy('dev864593051335148')
    pcks = b.egps_packages_range(
        datetime(2024, 10, 28, 12, 40, tzinfo=timezone.utc),
        datetime(2024, 10, 28, 13, 20, tzinfo=timezone.utc))

    c = EgpsCollection(pcks)
    ds = c.to_dataset()
    print(ds)

    ds = ds.sel(time=slice('2024-10-28T12:40', '2024-10-28T13:00'))

    if plot:
        plt.figure()
        z = ds.z / 1000.
        z.plot()
        plt.grid()
        plt.show()

    print(ds.z.mean())

    # assert ds.w_z.mean() == approx(9.81, abs=0.3)


KNOTS_TO_MS = 1852.0 / 3600.0
MAX_SPEED_KNOTS = 16.0


@needs_hub
def test_sfy4_01_egpsb_position(sfyhub, plot):
    """
    Validate egpsb position (lon, lat) packages from SFY4-01 during a boat test
    on 2026-05-19 ~17:00-18:00 UTC (19:00-20:00 GMT+2). The buoy was mounted on
    a boat with speeds up to 16 knots, with some stationary periods.
    """
    b = sfyhub.buoy('SFY4-01')
    start = datetime(2026, 5, 19, 17, 0, tzinfo=timezone.utc)
    end   = datetime(2026, 5, 19, 18, 0, tzinfo=timezone.utc)

    pcks = b.egps_packages_range(start, end, binary=True)
    assert len(pcks) > 0, "No egpsb packages found for SFY4-01"

    c = EgpsCollection(pcks)
    ds = c.to_dataset()

    # lat/lon in the dataset are in units of deg * 1e7
    lat_deg = ds.lat / 1.0e7  # degrees north
    lon_deg = ds.lon / 1.0e7  # degrees east

    # Positions are finite
    assert np.all(np.isfinite(lat_deg)), "NaN/Inf in latitude"
    assert np.all(np.isfinite(lon_deg)), "NaN/Inf in longitude"

    # lat/lon must be float64 – float32 ULP near 6e8 (deg*1e7) is ~64 units
    # which collapses consecutive GPS samples onto the same grid point
    assert ds.lat.dtype == np.float64, "lat must be float64 to avoid quantization noise"
    assert ds.lon.dtype == np.float64, "lon must be float64 to avoid quantization noise"

    # Track stays within the expected geographic area (western Norway coast)
    assert float(lat_deg.min()) > 60.0, "Latitude too far south"
    assert float(lat_deg.max()) < 61.0, "Latitude too far north"
    assert float(lon_deg.min()) >  4.5, "Longitude too far west"
    assert float(lon_deg.max()) <  6.0, "Longitude too far east"

    # Speed from velocity fields (vn, ve in mm/s) – max is 16 knots
    speed_ms = np.sqrt(ds.vn**2 + ds.ve**2) / 1000.0   # m/s
    speed_kt = speed_ms / KNOTS_TO_MS

    assert float(speed_kt.max()) <= MAX_SPEED_KNOTS + 1.0, \
        f"Max speed {float(speed_kt.max()):.1f} kt exceeds expected {MAX_SPEED_KNOTS} kt"

    # Some periods where the boat is (nearly) stationary
    assert np.any(speed_kt < 1.0), "No stationary periods found (expected some)"

    if plot:
        fig, axes = plt.subplots(1, 2, figsize=(12, 5))

        axes[0].plot(lon_deg, lat_deg, '.', markersize=1)
        axes[0].set_xlabel('Longitude [°E]')
        axes[0].set_ylabel('Latitude [°N]')
        axes[0].set_title('SFY4-01 track (egpsb)')
        axes[0].grid(True)
        axes[0].set_aspect('equal')

        axes[1].plot(ds.time, speed_kt, linewidth=0.5)
        axes[1].axhline(MAX_SPEED_KNOTS, color='r', linestyle='--',
                        label=f'{MAX_SPEED_KNOTS} kt limit')
        axes[1].set_xlabel('Time [UTC]')
        axes[1].set_ylabel('Speed [knots]')
        axes[1].set_title('SFY4-01 speed (egpsb)')
        axes[1].legend()
        axes[1].grid(True)

        plt.tight_layout()
        plt.show()
