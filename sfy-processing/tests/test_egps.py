import pytest
import numpy as np
from datetime import datetime, timezone
from sfy.egps import Egps, EgpsCollection
from sfy import signal as sfysignal
from sfy import xr as sfyxr
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

    # lat/lon are in decimal degrees
    lat_deg = ds.lat
    lon_deg = ds.lon

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

    # Per-package reference positions (decimal degrees, package dim)
    assert float(ds.pck_lat.min()) > 60.0
    assert float(ds.pck_lon.min()) >  4.5

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


@needs_hub
def test_sfy4_01_egpsb_spectrum(sfyhub, plot):
    """
    Compute elevation spectra from SFY4-01 egpsb velocities (2026-05-19 17:00-18:00 UTC).
    Velocities (mm/s) are integrated once via Welch to yield elevation spectra (m^2/Hz).
    Horizontal (vn, ve) and vertical (vz) components are plotted separately.
    """
    b = sfyhub.buoy('SFY4-01')
    start = datetime(2026, 5, 19, 17, 0, tzinfo=timezone.utc)
    end   = datetime(2026, 5, 19, 18, 0, tzinfo=timezone.utc)

    pcks = b.egps_packages_range(start, end, binary=True)
    assert len(pcks) > 0

    c = EgpsCollection(pcks)
    ds = c.to_dataset()
    freq = c.frequency  # ~14 Hz

    # Convert mm/s -> m/s
    vn = ds.vn.values / 1000.0
    ve = ds.ve.values / 1000.0
    vz = ds.vz.values / 1000.0

    # Welch with order=1: integrate velocity spectrum once to elevation spectrum
    nperseg = min(4096, len(vn) // 4)
    f, Pn = sfysignal.welch(freq, vn, nperseg=nperseg, order=1)
    f, Pe = sfysignal.welch(freq, ve, nperseg=nperseg, order=1)
    f, Pz = sfysignal.welch(freq, vz, nperseg=nperseg, order=1)

    # Spectra are real and non-negative (below Nyquist, skip DC bin)
    assert np.all(Pn[1:] >= 0)
    assert np.all(Pe[1:] >= 0)
    assert np.all(Pz[1:] >= 0)

    # Hm0 from vertical elevation spectrum should be in a plausible range for coastal Norway
    hm0_z = sfysignal.hm0(f, Pz)
    print(f'Hm0 (vertical): {hm0_z:.2f} m')
    assert 0.0 < hm0_z < 10.0, f'Hm0_z={hm0_z:.2f} m out of plausible range'

    if plot:
        fig, axes = plt.subplots(2, 1, figsize=(10, 8), sharex=True)

        # Horizontal spectra
        axes[0].semilogy(f[1:], Pn[1:], label='North (vn)')
        axes[0].semilogy(f[1:], Pe[1:], label='East (ve)')
        axes[0].set_ylabel('Elevation PSD [m²/Hz]')
        axes[0].set_title('SFY4-01 horizontal elevation spectra (egpsb, velocity-integrated)')
        axes[0].legend()
        axes[0].grid(True, which='both')
        axes[0].set_xlim(0.01, freq / 2)

        # Vertical spectrum
        axes[1].semilogy(f[1:], Pz[1:], label=f'Vertical (vz)  Hm0={hm0_z:.2f} m', color='C2')
        axes[1].set_xlabel('Frequency [Hz]')
        axes[1].set_ylabel('Elevation PSD [m²/Hz]')
        axes[1].set_title('SFY4-01 vertical elevation spectrum (egpsb, velocity-integrated)')
        axes[1].legend()
        axes[1].grid(True, which='both')
        axes[1].set_xlim(0.01, freq / 2)

        plt.tight_layout()
        plt.show()


@needs_hub
def test_sfy4_01_egpsb_stationary_spectrum(sfyhub, plot):
    """
    Compute 20-minute elevation spectra from SFY4-01 egpsb while the buoy is
    stationary (2026-05-20 01:00-04:00 local / 2026-05-19 23:00 - 2026-05-20
    02:00 UTC). Uses egps_spec_stats (velocity integrated once, order=1).
    """
    b = sfyhub.buoy('SFY4-01')
    # 01:00-04:00 local (GMT+2) = 23:00-02:00 UTC
    start = datetime(2026, 5, 19, 23, 0, tzinfo=timezone.utc)
    end   = datetime(2026, 5, 20,  2, 0, tzinfo=timezone.utc)

    pcks = b.egps_packages_range(start, end, binary=True)
    assert len(pcks) > 0

    c = EgpsCollection(pcks)
    ds = c.to_dataset()

    # Buoy should be stationary – max speed < 1 kt
    speed_kt = np.sqrt(ds.vn**2 + ds.ve**2) / 1000.0 / (1852 / 3600)
    assert float(speed_kt.max()) < 1.0, \
        f"Buoy not stationary: max speed {float(speed_kt.max()):.2f} kt"

    ss = sfyxr.egps_spec_stats(ds, window=20 * 60)
    print(ss)
    print(f"Hm0 (vertical): {ss.hm0.values}")

    assert len(ss.time) >= 3, "Expected at least 3 × 20-min windows in 3-hour window"
    assert np.all(np.isfinite(ss.hm0)), "NaN Hm0 values"
    assert np.all(ss.hm0 >= 0)

    if plot:
        f = ss.frequency.values
        fig, axes = plt.subplots(2, 1, figsize=(10, 8), sharex=True)

        for t in ss.time:
            label = str(t.values)[:16]
            axes[0].semilogy(f[1:], ss.En.sel(time=t).values[1:], alpha=0.7, label=label)
            axes[0].semilogy(f[1:], ss.Ee.sel(time=t).values[1:], alpha=0.7, linestyle='--')
        axes[0].set_ylabel('Elevation PSD [m²/Hz]')
        axes[0].set_title('SFY4-01 horizontal spectra – stationary (egpsb, 20-min windows)')
        axes[0].legend(fontsize=7)
        axes[0].grid(True, which='both')
        axes[0].set_xlim(0.01, c.frequency / 2)

        for t in ss.time:
            hm0_val = float(ss.hm0.sel(time=t).values)
            label = f"{str(t.values)[:16]}  Hm0={hm0_val:.2f}m"
            axes[1].semilogy(f[1:], ss.E.sel(time=t).values[1:], alpha=0.7, label=label)
        axes[1].set_xlabel('Frequency [Hz]')
        axes[1].set_ylabel('Elevation PSD [m²/Hz]')
        axes[1].set_title('SFY4-01 vertical spectrum – stationary (egpsb, 20-min windows)')
        axes[1].legend(fontsize=7)
        axes[1].grid(True, which='both')
        axes[1].set_xlim(0.01, c.frequency / 2)

        plt.tight_layout()
        plt.show()


@needs_hub
def test_sfy4_01_egpsb_vs_axlb_spectrum(sfyhub, plot):
    """
    Compare the first 20-min egpsb elevation spectrum (velocity integrated once)
    against the co-located axlb elevation spectrum (acceleration integrated twice)
    for SFY4-01 while stationary (2026-05-19 23:00 - 2026-05-20 02:00 UTC).
    Both spectra should agree in the wave band.
    """
    from sfy.axl import AxlCollection
    from sfy import xr as sfyxr

    b = sfyhub.buoy('SFY4-01')
    start = datetime(2026, 5, 19, 23, 0, tzinfo=timezone.utc)
    end   = datetime(2026, 5, 20,  2, 0, tzinfo=timezone.utc)

    # --- egpsb: first 20-min window ---
    epcks = b.egps_packages_range(start, end, binary=True)
    assert len(epcks) > 0
    ec = EgpsCollection(epcks)
    eds = ec.to_dataset()
    ess = sfyxr.egps_spec_stats(eds, window=20 * 60)

    f_egps = ess.frequency.values
    E_egps_z = ess.E.isel(time=0).values
    E_egps_n = ess.En.isel(time=0).values
    E_egps_e = ess.Ee.isel(time=0).values
    hm0_egps = float(ess.hm0.isel(time=0).values)
    print(f'egps Hm0: {hm0_egps:.3f} m')

    # --- axlb: same 20-min window ---
    apcks = b.axl_packages_range(start, end, binary=True)
    assert len(apcks) > 0
    ac = AxlCollection(apcks)
    ads = ac.to_dataset()

    # Clip axl dataset to the same first 20-min window as egps
    t_egps_win_end = ess.time.isel(time=0).values
    t_start_ns = np.datetime64(int(ec.start.timestamp() * 1e9), 'ns')
    ads_win = ads.sel(time=slice(t_start_ns, t_egps_win_end))
    ass_win = sfyxr.spec_stats(ads_win, window='full')

    f_axl = ass_win.frequency.values
    E_axl_z = ass_win.E.isel(time=0).values
    hm0_axl = float(ass_win.hm0.isel(time=0).values)
    print(f'axlb Hm0: {hm0_axl:.3f} m')

    # Both Hm0 should be finite and agree within a factor of 2
    assert np.isfinite(hm0_egps)
    assert np.isfinite(hm0_axl)
    assert abs(hm0_egps - hm0_axl) < max(hm0_egps, hm0_axl), \
        f"Hm0 mismatch: egps={hm0_egps:.3f} m  axlb={hm0_axl:.3f} m"

    if plot:
        fig, ax = plt.subplots(figsize=(10, 5))
        ax.semilogy(f_egps[1:], E_egps_z[1:], label=f'egpsb vertical  Hm0={hm0_egps:.3f} m', color='C2')
        ax.semilogy(f_egps[1:], E_egps_n[1:], label='egpsb north', color='C0', linestyle='--', alpha=0.7)
        ax.semilogy(f_egps[1:], E_egps_e[1:], label='egpsb east',  color='C1', linestyle='--', alpha=0.7)
        ax.semilogy(f_axl[1:],  E_axl_z[1:],  label=f'axlb vertical  Hm0={hm0_axl:.3f} m',  color='C2', linestyle=':', linewidth=2)
        ax.set_xlabel('Frequency [Hz]')
        ax.set_ylabel('Elevation PSD [m²/Hz]')
        ax.set_title('SFY4-01 egpsb vs axlb – first 20-min window (stationary)')
        ax.set_xlim(0.01, min(ec.frequency, ac.frequency) / 2)
        ax.legend()
        ax.grid(True, which='both')
        plt.tight_layout()
        plt.show()