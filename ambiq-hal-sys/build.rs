extern crate bindgen;

use cc;
use glob;
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Build the Board Support Crate for the desired chip. We're starting out with
    // "Sparkfun Redboard Artemis".
    //
    // TODO: Choose the correct board depending on feature flag.
    println!("Building the Sparkfun BSP");

    env::var_os("CARGO_FEATURE_SPARKFUN_REDBOARD")
        .expect("Only the Sparkfun Artemis Redboard is supported");

    Command::new("make")
        .current_dir("ambiq-sparkfun-sdk/boards_sfe/redboard_artemis/bsp/gcc")
        .status()
        .expect("could not re-build the BSP library");

    Command::new("make")
        .current_dir("ambiq-sparkfun-sdk/mcu/apollo3/hal/gcc")
        .status()
        .expect("could not re-build the HAL library");

    // The BSP library appears to be statically linked to the am_hal library containing the
    // apollo3 MCU functions (modulo the current chip + MCU).
    println!("cargo:rustc-link-lib=static=am_bsp");
    println!("cargo:rustc-link-lib=static=am_hal");
    println!(
        "cargo:rustc-link-search=native=ambiq-sparkfun-sdk/boards_sfe/redboard_artemis/bsp/gcc/bin"
    );
    println!("cargo:rustc-link-search=native=ambiq-sparkfun-sdk/mcu/apollo3/hal/gcc/bin");
    println!("cargo:lib=am_bsp");
    println!("cargo:lib=am_hal");

    // Entry-point
    cc::Build::new()
        .file("ambiq-sparkfun-sdk/boards_sfe/common/tools_sfe/templates/startup_gcc.c")
        .compile("startup_gcc");

    // Utils
    let mut compiler = cc::Build::new();
    compiler.include("ambiq-sparkfun-sdk/mcu/apollo3");
    compiler.include("ambiq-sparkfun-sdk/CMSIS/AmbiqMicro/Include");
    compiler.include("ambiq-sparkfun-sdk/CMSIS/ARM/Include");
    compiler.include("ambiq-sparkfun-sdk/devices");

    for path in glob::glob("ambiq-sparkfun-sdk/utils/*.c").unwrap() {
        let path = path.unwrap();
        if !path.file_name().unwrap().to_str().unwrap().ends_with("regdump.c") {
            compiler.file(path);
        }
    }
    compiler.compile("am_utils");

    // Devices
    let mut compiler = cc::Build::new();
    compiler.include("ambiq-sparkfun-sdk/mcu/apollo3");
    compiler.include("ambiq-sparkfun-sdk/CMSIS/AmbiqMicro/Include");
    compiler.include("ambiq-sparkfun-sdk/CMSIS/ARM/Include");
    compiler.include("ambiq-sparkfun-sdk/devices");
    compiler.include("ambiq-sparkfun-sdk/boards_sfe/redboard_artemis/bsp");

    let paths = &[
        "am_devices_button.c",
        "am_devices_led.c",
    ];

    for path in paths {
        let path = PathBuf::from("ambiq-sparkfun-sdk/devices/").join(&path);
        compiler.file(path);
    }
    compiler.compile("am_devices");

    println!("cargo:rerun-if-changed=wrapper.h");
    // println!("cargo:rerun-if-changed=build.rs");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .use_core()
        .ctypes_prefix("c_types")
        .clang_arg("-Iambiq-sparkfun-sdk/boards_sfe/redboard_artemis/bsp")
        .clang_arg("-Iambiq-sparkfun-sdk/mcu/apollo3")
        .clang_arg("-Iambiq-sparkfun-sdk/CMSIS/AmbiqMicro/Include")
        .clang_arg("-Iambiq-sparkfun-sdk/CMSIS/ARM/Include")
        .clang_arg("-Iambiq-sparkfun-sdk/devices")
        .clang_arg("-Iambiq-sparkfun-sdk/utils")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
