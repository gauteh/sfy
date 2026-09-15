//! Lightweight duty-cycle/health counters, reported and reset once per
//! `sync_period` (see `report_and_reset`, called from the main loop and
//! pushed out via `sfy::log`).
//!
//! Kept intentionally minimal given limited field-testing time: plain
//! relaxed atomics incremented from wherever the corresponding event
//! already happens, no locking -- exact interleavings don't matter for a
//! coarse per-period summary like this.

use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};
use heapless::String;

/// Number of axl (IMU/wave) packets successfully queued for sending.
pub static AXL_PACKETS: AtomicU32 = AtomicU32::new(0);

/// Number of egps (GPS raw sample batch) packets successfully queued for sending.
pub static EGPS_PACKETS: AtomicU32 = AtomicU32::new(0);

/// Number of times the egps duty-cycle gave up re-initialising the GPS
/// module after exhausting its retry budget for a wake (see
/// `EgpsAction::WakePowerOnReinit` in `sfy4-main`).
pub static GPS_REINIT_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Number of times a fresh egps fix's PPS-vs-now `diff` was out of range and
/// therefore not used to (re-)sync the RTC (see `Location::set_from_egps`).
/// A high count here means batches are likely being starved even with good
/// reception -- see the duty-cycle discussion around `fix_acquired`.
pub static GPS_PPS_DIFF_REJECTED: AtomicU32 = AtomicU32::new(0);

/// Number of IMU `check_retrieve` failures (RTC ISR).
pub static IMU_FAILURES: AtomicU32 = AtomicU32::new(0);

fn take(counter: &AtomicU32) -> u32 {
    counter.swap(0, Ordering::Relaxed)
}

fn peek(counter: &AtomicU32) -> u32 {
    counter.load(Ordering::Relaxed)
}

fn format(axl: u32, egps: u32, reinit_fail: u32, pps_rej: u32, imu_fail: u32) -> String<128> {
    let mut s = String::new();
    write!(
        s,
        "status: axl={} egps={} gps_reinit_fail={} pps_rej={} imu_fail={}",
        axl, egps, reinit_fail, pps_rej, imu_fail
    )
    .ok();
    s
}

/// Format a compact summary of all counters since the last call, and reset
/// them for the next period. Returns `None` if nothing at all happened
/// (fully idle/all-zero) to avoid spamming the log with empty statuses.
pub fn report_and_reset() -> Option<String<128>> {
    let axl = take(&AXL_PACKETS);
    let egps = take(&EGPS_PACKETS);
    let reinit_fail = take(&GPS_REINIT_FAILURES);
    let pps_rej = take(&GPS_PPS_DIFF_REJECTED);
    let imu_fail = take(&IMU_FAILURES);

    if axl == 0 && egps == 0 && reinit_fail == 0 && pps_rej == 0 && imu_fail == 0 {
        return None;
    }

    Some(format(axl, egps, reinit_fail, pps_rej, imu_fail))
}

/// Format the current counters *without* resetting them -- for cheap,
/// frequent (e.g. every main-loop iteration) console/RTT visibility via
/// `defmt`, independent of the periodic `report_and_reset` sent over the
/// Notecard link.
pub fn format_status() -> String<128> {
    format(
        peek(&AXL_PACKETS),
        peek(&EGPS_PACKETS),
        peek(&GPS_REINIT_FAILURES),
        peek(&GPS_PPS_DIFF_REJECTED),
        peek(&IMU_FAILURES),
    )
}
