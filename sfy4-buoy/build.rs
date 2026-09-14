use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("config.rs");

    let sync_period: u32 = option_env!("SYNC_PERIOD")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(20);

    // egps duty-cycle: the GPS module always runs this same state machine
    // (see `sfy::gps::duty`); there is no separate feature flag for it any
    // more. `EGPS_POSITION_INTERVAL` (the minimum position/time-fix cadence)
    // defaults to `0`, meaning "no gap" -- the module is never idled/power
    // cycled and just runs continuously (matching the historical
    // always-on default). Set it to a non-zero number of seconds to start
    // duty-cycling: the module wakes for a position/time fix every
    // `EGPS_POSITION_INTERVAL`, and separately runs a fixed 20-minute
    // high-rate batch (IMU/GPS, see `EGPS_BATCH_DURATION_S`) every
    // `EGPS_BATCH_PERIOD` (start-to-start).
    let egps_position_interval: u32 = option_env!("EGPS_POSITION_INTERVAL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(0); // 0 = continuous (no duty-cycle gap), the historical default

    let egps_position_dwell: u32 = option_env!("EGPS_POSITION_DWELL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(480);

    let egps_batch_period: u32 = option_env!("EGPS_BATCH_PERIOD")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(10800); // 3 h

    if egps_position_interval > 0 && egps_position_dwell > egps_position_interval {
        println!(
            "cargo:warning=EGPS_POSITION_DWELL ({egps_position_dwell}s) is greater than \
             EGPS_POSITION_INTERVAL ({egps_position_interval}s); a stuck fix acquisition could \
             overrun into the next scheduled wake."
        );
    }

    // Batch duration is fixed at 20 min (`sfy::gps::duty::EGPS_BATCH_DURATION_S`),
    // not build-time configurable -- kept in sync with that constant here for
    // the sanity check below.
    const EGPS_BATCH_DURATION_S: u32 = 1200;
    if egps_batch_period < EGPS_BATCH_DURATION_S + egps_position_interval {
        println!(
            "cargo:warning=EGPS_BATCH_PERIOD ({egps_batch_period}s) is shorter than \
             EGPS_BATCH_DURATION_S + EGPS_POSITION_INTERVAL ({}s); batches will not be \
             spaced as configured (they may run back-to-back or start on every position wake).",
            EGPS_BATCH_DURATION_S + egps_position_interval
        );
    }

    // --- Remote power-mode ladder (Notehub env vars `power_mode` / `sync_period`) ---
    //
    // `power_mode` (0-3) and `sync_period` are live-adjustable at runtime via Notehub
    // environment variables (see `sfy::power`). The knobs below are the per-level
    // *tunables* for that ladder -- they stay build-time-configurable so the ladder
    // itself can be tuned per-deployment without adding more runtime knobs.
    //
    // Levels 0 (Normal), 1 (NoBatch) and 2 (DutyImu) all reuse the egps duty-cycle
    // knobs above -- level 2 differs only in that IMU/AXL streaming is confined to the
    // egps batch window (always `EGPS_BATCH_DURATION_S`) instead of running
    // continuously, so there is much less data to sync.

    // Default power mode to start in before the first successful env.get (0 = Normal).
    let power_mode: u8 = option_env!("POWER_MODE")
        .map(|p| p.parse::<u8>().unwrap())
        .unwrap_or(0);

    // How often (seconds) the main loop polls Notehub env vars for power-mode/sync-period
    // changes. Kept fairly infrequent since this is a low-frequency control-plane check.
    let env_poll_interval: u32 = option_env!("ENV_POLL_INTERVAL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(1200); // 20 min

    // Level 3 (PositionOnly): egps wakes only for a position fix every
    // `egps_l3_position_interval` seconds (no batch, no continuous streaming),
    // waiting up to `egps_l3_position_dwell` seconds for a fix, IMU sampling is fully
    // disabled, and a sync is forced after each wake attempt regardless of whether a
    // fix was obtained (handled in firmware, not via `sync_period`).
    let egps_l3_position_interval: u32 = option_env!("EGPS_L3_POSITION_INTERVAL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(43200); // 12 h

    let egps_l3_position_dwell: u32 = option_env!("EGPS_L3_POSITION_DWELL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(120);

    if egps_l3_position_dwell > egps_l3_position_interval {
        println!(
            "cargo:warning=EGPS_L3_POSITION_DWELL ({egps_l3_position_dwell}s) is greater than \
             EGPS_L3_POSITION_INTERVAL ({egps_l3_position_interval}s); a stuck fix acquisition \
             could overrun into the next scheduled wake."
        );
    }

    let fd = fs::File::create(&dest_path).unwrap();
    writeln!(&fd, "pub const SYNC_PERIOD: u32 = {sync_period};").unwrap();
    writeln!(
        &fd,
        "pub const EGPS_POSITION_INTERVAL: u32 = {egps_position_interval};"
    )
    .unwrap();
    writeln!(
        &fd,
        "pub const EGPS_POSITION_DWELL: u32 = {egps_position_dwell};"
    )
    .unwrap();
    writeln!(
        &fd,
        "pub const EGPS_BATCH_PERIOD: u32 = {egps_batch_period};"
    )
    .unwrap();

    writeln!(&fd, "pub const POWER_MODE: u8 = {power_mode};").unwrap();
    writeln!(&fd, "pub const ENV_POLL_INTERVAL: u32 = {env_poll_interval};").unwrap();
    writeln!(
        &fd,
        "pub const EGPS_L3_POSITION_INTERVAL: u32 = {egps_l3_position_interval};"
    )
    .unwrap();
    writeln!(
        &fd,
        "pub const EGPS_L3_POSITION_DWELL: u32 = {egps_l3_position_dwell};"
    )
    .unwrap();

    if option_env!("BUOYSN").is_none() {
        println!("cargo:warning=BUOYSN: No buoy name supplied, using device id or previously configured.");
    }

    if option_env!("BUOYPR").is_none() {
        println!(
            "cargo:warning=BUOYPR: No notehub modem product supplied, using previously configured."
        );
    }

    println!("cargo:rerun-if-changed=build.rs");
}
