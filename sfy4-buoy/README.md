# sfy-buoy (small friendly buoy)

Folders:

* sfy - library of firmware, portable to different platforms + tool for
    unpacking SD-card files.
* sfy4-main - main function targeted for the Artemis.
* target-test - unit tests for Artemis.

## Building for deployment
```sh
$ BUOYPR=xxxx:your-notehub-account BUOYSN=WAVEBUGXX DEFMT_LOG=debug make T=r
```

You can flash the firmware using the USB bootloader:

```sh
$ make T=r flash
```

## Dependencies

Tested on Ubuntu 20 and 22:

```sh
# ARM cross-compiler + binutils
sudo apt install gcc-arm-none-eabi binutils-arm-none-eabi

# C standard library headers and runtime for bare-metal ARM
sudo apt install libnewlib-arm-none-eabi

# clang / libclang (needed by bindgen in ahrs-fusion and similar crates)
sudo apt install clang libclang-dev

# Rust toolchain
rustup target add thumbv7em-none-eabihf
rustup component add rust-src llvm-tools-preview rustc-dev
cargo install cargo-binutils
```

### Ubuntu 24

Ubuntu 24 ships clang 18 by default, which is incompatible with the `bindgen`
version used by `ambiq-hal-sys`. Install clang 14 and set two environment
variables before building:

```sh
sudo apt install clang-14
```

```sh
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
export BINDGEN_EXTRA_CLANG_ARGS="--target=thumbv7em-none-eabihf -I/usr/lib/gcc/arm-none-eabi/13.2.1/include"
```

- `LIBCLANG_PATH` forces bindgen to use clang 14 instead of clang 18.
- `BINDGEN_EXTRA_CLANG_ARGS` points clang at the ARM cross-compiler headers
  (Ubuntu 24 ships GCC ARM 13.x at a different path than earlier releases).

### Running host tests

Host tests run on the development machine (no hardware required):

```sh
make host-test
```

## Hardware debugger

With the Artemis the JLink EDU debugger works fairly well, install the
[debug-server and tools](https://www.segger.com/downloads/jlink/).

### Debugging the Notecard

The notecard outputs debug information on the USB-TTY (using e.g. `picocom` with
baud rate: 115200). Type `trace` + Enter to get much more information. If you
want to get debug information without powering the whole system through the
USB-port you have to attach a [FTDI-RS232 adapter to
AUXRX/AUXTX and pull AUXEN up](https://dev.blues.io/guides-and-tutorials/notecard-guides/debugging-with-the-ftdi-debug-cable/).

## Feature flags and environment variables

### Features

* defmt-serial (experimental): logs defmt-messages over serial rather than RTT. So that you can
    read messages without a hardware-debugger. See `make defmt-serial`.

* continuous: transmits data continuously, at the cost of more power and no
    functional GPS. Mostly for demonstration purposes.

* 20Hz: set output sample rate of waves to 20Hz, rather than 52hz.

* deploy: turns on `asm::wfi` in main loop over busy wait.

* storage: store data on SD card.

* fir: recommended and sometimes needed: run IMU faster and filter kalman-output down to output
    rate.

* surf: increase accel and gyro range to expect greater forces impacted by
    breaking waves.

* ice: increase sensitivity (opposite of surf), expect low movement and low
    forces. typically used for ice deployments.

* surf: increase accel and gyro range to expect greater forces impacted by
    breaking waves.

* raw: store raw data on SD-card (experimental)

* host-tests: used to disable code that doesn't compile on host, for running
    host unit tests. Best used through `make host-test`.

* egps-duty-cycle: duty-cycle the external GPS (egps, MAX-M10S) instead of
    running it continuously: wake only periodically for a position/time fix,
    and optionally run a high-rate "egps spectrum" burst on its own duty
    cycle (used to derive a wave spectrum from GPS displacement, sent as
    `egpsb.qo`). Setting `EGPS_SPECTRUM_DURATION=0` disables bursts entirely
    (position-only mode). See the env vars below.

    **This feature is off by default.** To get the current/original
    behavior -- egps running continuously at full rate, always streaming
    `egpsb.qo` -- simply do not enable this feature (i.e. build without
    `--features egps-duty-cycle`); nothing else needs to change.

    Each spectrum burst always runs for exactly `EGPS_SPECTRUM_DURATION`
    seconds (~20 minutes by default) once it starts -- this is enforced by
    the state machine itself (measured from when a fix is acquired) and is
    independent of `EGPS_SPECTRUM_PERIOD`. `EGPS_SPECTRUM_PERIOD` only
    controls how often a *new* burst is started (start-to-start interval);
    it does not affect a burst's length. If `EGPS_SPECTRUM_PERIOD` is set
    shorter than `EGPS_SPECTRUM_DURATION + EGPS_POSITION_INTERVAL`, bursts
    will not be spaced as configured (they can run back-to-back, or start
    on every position wake) -- `build.rs` emits a `cargo:warning` at build
    time if this is misconfigured.

### Environment variables

* BUOYSN: the name of the buoy as it appears on the data server.

* BUOYPR: the product name used for the modem. determines which account the data
    is sent to on notehub.io.

* SFY_EXT_SIM_APN: Enable external SIM and specify APN.

* GPS_PERIOD: Sample interval for GPS (default 60 seconds).

* GPS_HEARTBEAT: Minimum GPS interval when no motion detected. Positive value is
    hours, negative is minutes.

* SYNC_PERIOD: Maximum time between syncs (default 20 minutes).

* DEFMT_LOG: defmt log levels, leave empty to compile out.

* EGPS_POSITION_INTERVAL (feature `egps-duty-cycle`): how often to wake the
    egps module for a position/time fix outside of a spectrum burst, in
    seconds (default 600, 10 minutes).

* EGPS_POSITION_DWELL (feature `egps-duty-cycle`): max time to wait for a
    valid fix per wake before giving up and going back to idle, in seconds
    (default 120, 2 minutes).

* EGPS_SPECTRUM_DURATION (feature `egps-duty-cycle`): length of a high-rate
    burst during which raw samples are collected and sent as `egpsb.qo`, in
    seconds (default 1200, 20 minutes). Always honoured exactly (measured
    from fix acquisition to burst end). `0` disables bursts entirely
    (position-only mode).

* EGPS_SPECTRUM_PERIOD (feature `egps-duty-cycle`): start-to-start interval
    between spectrum bursts, in seconds (default 10800, 3 hours). Must be
    >= `EGPS_SPECTRUM_DURATION + EGPS_POSITION_INTERVAL` or bursts won't be
    spaced as configured (a build-time warning is emitted otherwise).

* EGPS_SLEEP_THRESHOLD (feature `egps-duty-cycle`): idle gaps less than or
    equal to this use UBX backup sleep (module stays powered, fast resume);
    gaps above this fully power off the module via the `d8` GPIO
    (near-zero standby current, needs re-init on wake), in seconds
    (default 1800, 30 minutes).

# Troubleshooting

1. On Ubuntu 22 the package `brltty` claims the Artemis USB device and the tty
   device disappears, remove it if you don't need it.
