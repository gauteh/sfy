use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("config.rs");

    let gps_period: u32 = option_env!("GPS_PERIOD")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(60);

    let gps_heartbeat: i32 = option_env!("GPS_HEARTBEAT")
        .map(|p| p.parse::<i32>().unwrap())
        .unwrap_or(1);

    let sync_period: u32 = option_env!("SYNC_PERIOD")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(20);

    // egps duty-cycle mode (feature `egps-duty-cycle`): position fix interval,
    // max dwell time waiting for a fix, spectrum burst duration/period, and
    // the idle-gap threshold between UBX backup sleep and full power-off.
    let egps_position_interval: u32 = option_env!("EGPS_POSITION_INTERVAL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(600);

    let egps_position_dwell: u32 = option_env!("EGPS_POSITION_DWELL")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(120);

    let egps_spectrum_duration: u32 = option_env!("EGPS_SPECTRUM_DURATION")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(1200);

    let egps_spectrum_period: u32 = option_env!("EGPS_SPECTRUM_PERIOD")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(10800);

    let egps_sleep_threshold: u32 = option_env!("EGPS_SLEEP_THRESHOLD")
        .map(|p| p.parse::<u32>().unwrap())
        .unwrap_or(1800);

    // Sanity-check the duty-cycle knobs: EGPS_SPECTRUM_DURATION is always
    // honoured exactly by the state machine (a burst always runs for exactly
    // that long once started), but EGPS_SPECTRUM_PERIOD is only the
    // start-to-start interval *between* bursts. If it's shorter than a
    // burst's duration plus the position-fix interval needed to start the
    // next one, bursts will not be spaced as configured -- they may run
    // back-to-back or start on every position wake.
    if egps_spectrum_duration > 0
        && egps_spectrum_period < egps_spectrum_duration + egps_position_interval
    {
        println!(
            "cargo:warning=EGPS_SPECTRUM_PERIOD ({egps_spectrum_period}s) is shorter than \
             EGPS_SPECTRUM_DURATION + EGPS_POSITION_INTERVAL ({}s); spectrum bursts will not be \
             spaced as configured (they may run back-to-back or start on every position wake).",
            egps_spectrum_duration + egps_position_interval
        );
    }

    if egps_position_dwell > egps_position_interval {
        println!(
            "cargo:warning=EGPS_POSITION_DWELL ({egps_position_dwell}s) is greater than \
             EGPS_POSITION_INTERVAL ({egps_position_interval}s); a stuck fix acquisition could \
             overrun into the next scheduled wake."
        );
    }

    let fd = fs::File::create(&dest_path).unwrap();
    writeln!(&fd, "pub const GPS_PERIOD: u32 = {gps_period};").unwrap();
    writeln!(&fd, "pub const GPS_HEARTBEAT: i32 = {gps_heartbeat};").unwrap();
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
        "pub const EGPS_SPECTRUM_DURATION: u32 = {egps_spectrum_duration};"
    )
    .unwrap();
    writeln!(
        &fd,
        "pub const EGPS_SPECTRUM_PERIOD: u32 = {egps_spectrum_period};"
    )
    .unwrap();
    writeln!(
        &fd,
        "pub const EGPS_SLEEP_THRESHOLD: u32 = {egps_sleep_threshold};"
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
