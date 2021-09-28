extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Build the Board Support Crate for the desired chip. We're starting out with
    // "Sparkfun Redboard Artemis".
    //
    // TODO: Choose the correct board depending on feature flag.
    println!("Building the Sparkfun BSP");

    env::var_os("CARGO_FEATURE_SPARKFUN_REDBOARD").expect("Only the Sparkfun Artemis Redboard is supported");

    Command::new("make")
        .current_dir("ambiq-sparkfun-sdk/boards_sfe/redboard_artemis/bsp/gcc")
        .status().expect("could not re-build the BSP library");


    // The BSP library appears to be statically linked to the am_hal library containing the
    // apollo3 MCU functions (modulo the current chip + MCU).
    println!("cargo:rustc-link-lib=static=am_bsp");
    println!("cargo:rustc-link-search=native=ambiq-sparkfun-sdk/boards_sfe/redboard_artemis/bsp/gcc/bin");

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
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
