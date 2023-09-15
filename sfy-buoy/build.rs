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

    let fd = fs::File::create(&dest_path).unwrap();
    writeln!(&fd, "pub const GPS_PERIOD: u32 = {gps_period};").unwrap();
    writeln!(&fd, "pub const GPS_HEARTBEAT: i32 = {gps_heartbeat};").unwrap();
    writeln!(&fd, "pub const SYNC_PERIOD: u32 = {sync_period};").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
