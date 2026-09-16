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

/// Number of times the egps duty-cycle attempted a wake (`WakePowerOnReinit`,
/// successful or not) -- see `apply_egps_action` in `sfy4-main`. Compare
/// against `egps_reinit_fail`/`egps_dwell_timeout`/`egps` to see how many
/// wakes actually made it to a re-init, a fix, or neither.
pub static EGPS_WAKE_ATTEMPTS: AtomicU32 = AtomicU32::new(0);

/// Number of times the egps duty-cycle gave up re-initialising the GPS
/// module after exhausting its retry budget for a wake (see
/// `EgpsAction::WakePowerOnReinit` in `sfy4-main`).
pub static GPS_REINIT_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Number of times a wake's fix acquisition dwell timed out (module
/// re-init succeeded, but no valid date+time fix arrived within
/// `position_dwell_ms`) -- see `EgpsState::AcquiringFix` in
/// `sfy::gps::duty`. A high count with `gps_reinit_fail == 0` means the GPS
/// module itself is responding fine but isn't achieving a fix in time
/// (e.g. poor antenna/RF reception), rather than a hardware/I2C fault.
pub static EGPS_DWELL_TIMEOUT: AtomicU32 = AtomicU32::new(0);

/// Number of times a fresh egps fix's PPS-vs-now `diff` was out of range and
/// therefore not used to (re-)sync the RTC (see `Location::set_from_egps`).
/// A high count here means batches are likely being starved even with good
/// reception -- see the duty-cycle discussion around `fix_acquired`.
pub static GPS_PPS_DIFF_REJECTED: AtomicU32 = AtomicU32::new(0);

/// Number of `read_all_pvts` I2C failures while draining the GPS FIFO (RTC
/// ISR) -- indicates a flaky I2C bus/module rather than simply "no fix yet".
pub static GPS_I2C_ERRORS: AtomicU32 = AtomicU32::new(0);

/// Number of times `STATE.now()` (RTC read) failed in the main loop and the
/// last known-good time was reused instead (see the main loop's
/// `last_known_now_ms` fallback).
pub static RTC_READ_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Number of times the main loop fell back to the Notecard's own
/// location/time service because egps hadn't (re-)synced the RTC within
/// `EGPS_RTC_FALLBACK_TIMEOUT` (see `Location::check_retrieve`).
pub static NOTECARD_RTC_FALLBACK: AtomicU32 = AtomicU32::new(0);

/// Number of IMU `check_retrieve` failures (RTC ISR).
pub static IMU_FAILURES: AtomicU32 = AtomicU32::new(0);

fn take(counter: &AtomicU32) -> u32 {
    counter.swap(0, Ordering::Relaxed)
}

fn peek(counter: &AtomicU32) -> u32 {
    counter.load(Ordering::Relaxed)
}

#[allow(clippy::too_many_arguments)]
fn format(
    axl: u32,
    egps: u32,
    egps_wake: u32,
    reinit_fail: u32,
    dwell_timeout: u32,
    pps_rej: u32,
    gps_i2c_err: u32,
    rtc_read_fail: u32,
    rtc_fallback: u32,
    imu_fail: u32,
) -> String<256> {
    let mut s = String::new();
    write!(
        s,
        "status: axl={} egps={} egps_wake={} gps_reinit_fail={} egps_dwell_timeout={} \
         pps_rej={} gps_i2c_err={} rtc_read_fail={} rtc_fallback={} imu_fail={}",
        axl,
        egps,
        egps_wake,
        reinit_fail,
        dwell_timeout,
        pps_rej,
        gps_i2c_err,
        rtc_read_fail,
        rtc_fallback,
        imu_fail
    )
    .ok();
    s
}

/// Format a compact summary of all counters since the last call, and reset
/// them for the next period. Returns `None` if nothing at all happened
/// (fully idle/all-zero) to avoid spamming the log with empty statuses.
pub fn report_and_reset() -> Option<String<256>> {
    let axl = take(&AXL_PACKETS);
    let egps = take(&EGPS_PACKETS);
    let egps_wake = take(&EGPS_WAKE_ATTEMPTS);
    let reinit_fail = take(&GPS_REINIT_FAILURES);
    let dwell_timeout = take(&EGPS_DWELL_TIMEOUT);
    let pps_rej = take(&GPS_PPS_DIFF_REJECTED);
    let gps_i2c_err = take(&GPS_I2C_ERRORS);
    let rtc_read_fail = take(&RTC_READ_FAILURES);
    let rtc_fallback = take(&NOTECARD_RTC_FALLBACK);
    let imu_fail = take(&IMU_FAILURES);

    if axl == 0
        && egps == 0
        && egps_wake == 0
        && reinit_fail == 0
        && dwell_timeout == 0
        && pps_rej == 0
        && gps_i2c_err == 0
        && rtc_read_fail == 0
        && rtc_fallback == 0
        && imu_fail == 0
    {
        return None;
    }

    Some(format(
        axl,
        egps,
        egps_wake,
        reinit_fail,
        dwell_timeout,
        pps_rej,
        gps_i2c_err,
        rtc_read_fail,
        rtc_fallback,
        imu_fail,
    ))
}

/// Format the current counters *without* resetting them -- for cheap,
/// frequent (e.g. every main-loop iteration) console/RTT visibility via
/// `defmt`, independent of the periodic `report_and_reset` sent over the
/// Notecard link.
pub fn format_status() -> String<256> {
    format(
        peek(&AXL_PACKETS),
        peek(&EGPS_PACKETS),
        peek(&EGPS_WAKE_ATTEMPTS),
        peek(&GPS_REINIT_FAILURES),
        peek(&EGPS_DWELL_TIMEOUT),
        peek(&GPS_PPS_DIFF_REJECTED),
        peek(&GPS_I2C_ERRORS),
        peek(&RTC_READ_FAILURES),
        peek(&NOTECARD_RTC_FALLBACK),
        peek(&IMU_FAILURES),
    )
}
