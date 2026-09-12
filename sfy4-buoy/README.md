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

### GPS duty-cycle and power modes (WIP)

The external GPS (egps, MAX-M10S) is managed by a state machine
(`sfy::gps::duty::EgpsDutyCycle`) that is always compiled in (no feature
flag): it cycles the module between idle, fix-acquisition, and (optionally) a
high-rate "batch" burst during which raw samples are collected and sent as
`egpsb.qo`. This is separate from and unrelated to the axl/IMU `spectrum`
feature (FFT/Welch spectra of wave motion) -- the "batch" naming was chosen
specifically to avoid confusion with that feature.

By default `EGPS_POSITION_INTERVAL` is `0` ("no gap"), which reproduces the
historical always-on behavior: the module is never idled or power-cycled.
Set it to a non-zero value (seconds) to actually duty-cycle the module
between fixes, trading GPS availability/latency for power. See the
`EGPS_*` environment variables below for the individual knobs.

A remote `power_mode` Notehub env var additionally selects one of four power
levels (`sfy::power::PowerMode`), polled every `ENV_POLL_INTERVAL` seconds:

| Level | Name | Effect |
|-------|------|--------|
| 0 | Normal | Full `EGPS_*`-configured duty-cycle (position wake + periodic batch), IMU/AXL sampled and sent continuously. |
| 1 | NoBatch | Egps switches to position-only (no batches at all). IMU/AXL unchanged (continuous). |
| 2 | DutyImu | Same egps config as Normal (batches still happen), but IMU/AXL streaming is confined to exactly those batch windows instead of running continuously (`ImuMode::FollowEgpsBatch`) -- much less data, so `sync_period` can be relaxed. |
| 3 | PositionOnly | Egps wakes only for a position fix (`EGPS_L3_POSITION_INTERVAL`/`_DWELL`, ~12h), no batch, no continuous IMU streaming; a sync is forced after each wake attempt regardless of `sync_period`. |

The axl `spectrum` feature (Welch/FFT spectra of the IMU data) uses the same
`streaming` gate as the main IMU queue (see `Imu::check_retrieve`), so it is
already subject to the same duty-cycle-driven gating described above.

### Environment variables

* BUOYSN: the name of the buoy as it appears on the data server.

* BUOYPR: the product name used for the modem. determines which account the data
    is sent to on notehub.io.

* SFY_EXT_SIM_APN: Enable external SIM and specify APN.

* SYNC_PERIOD: Maximum time between syncs (default 20 minutes).

* DEFMT_LOG: defmt log levels, leave empty to compile out.

* EGPS_POSITION_INTERVAL: how often to wake the egps module for a
    position/time fix outside of a batch, in seconds. **Default `0`
    ("no gap"): the module is never idled or power-cycled and just runs
    continuously** -- this reproduces the historical always-on behavior.
    Set to a non-zero value to duty-cycle the module instead.

* EGPS_POSITION_DWELL: max time to wait for a valid fix per wake before
    giving up and going back to idle, in seconds (default 120, 2 minutes).
    Not used while `EGPS_POSITION_INTERVAL` is `0`.

Batches (raw samples collected and sent as `egpsb.qo`) always run for a
fixed 20 minutes once started -- not build-time configurable (matches the
axl/IMU `spectrum` feature's fixed 20-minute window). This is always
honoured exactly (measured from fix acquisition to burst end), except while
`EGPS_POSITION_INTERVAL` is `0`, in which case the burst just runs forever.
Batches are disabled entirely (position-only mode) at runtime via the
`power_mode` env var (see below), not at build time.

Position-only wakes (no batch due) still stream briefly right after the fix
is acquired -- just long enough to capture one packet's worth of raw
samples (`EGPS_POSITION_SAMPLE_MS` in `gps::duty`, not build-time
configurable) -- so something is queued and sent on the next sync instead
of the wake producing no data at all.

* EGPS_BATCH_PERIOD: start-to-start interval between batches, in
    seconds (default 10800, 3 hours). Must be >= 20 min (the fixed batch
    duration) + `EGPS_POSITION_INTERVAL` or bursts won't be spaced as
    configured (a build-time warning is emitted otherwise). Not used while
    `EGPS_POSITION_INTERVAL` is `0`.

Every idle gap fully powers the GPS module off via the `d8` GPIO
(near-zero standby current) and re-initializes it from scratch on the next
wake. There is no UBX backup-sleep idle mode: the MAX-M10S's
`UBX-RXM-PMREQ` backup sleep can only be woken by a hardware EXTINT pulse
or a full power cycle, and this board has no EXTINT pin wired up -- an
earlier attempt to use backup sleep (with a threshold-based choice between
backup sleep and power-off) left the module permanently unresponsive after
its first idle transition in the field, since nothing could ever wake it
back up.

* POWER_MODE: default remote power-mode level (0-3) to start in before the
    first successful `power_mode` Notehub env-var fetch (default 0,
    Normal). See `sfy::power`.

* ENV_POLL_INTERVAL: how often (seconds) the main loop polls the
    `power_mode`/`sync_period` Notehub env vars for live changes (default
    1200, 20 minutes).

* EGPS_L3_POSITION_INTERVAL: position-fix interval (seconds) used by power
    level 3 (`PositionOnly`) (default 43200, 12 hours).

* EGPS_L3_POSITION_DWELL: max time to wait for a fix (seconds) used by
    power level 3 (`PositionOnly`) (default 120, 2 minutes).

# Troubleshooting

1. On Ubuntu 22 the package `brltty` claims the Artemis USB device and the tty
   device disappears, remove it if you don't need it.
