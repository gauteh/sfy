# sfy-buoy (small friendly buoy)

Folders:

* sfy - library of firmware, portable to different platforms + tool for
    unpacking SD-card files.
* sfy-artemis - main function targeted for the Artemis.
* target-test - unit tests for Artemis.

usually flashing of the device etc. will be run from this directory.

## Building for deployment
```sh
$ cd sfy-artemis
$ BUOYPR=xxxx:your-notehub-account BUOYSN=WAVEBUGXX DEFMT_LOG=debug cargo build --release --features deploy
```

the `deploy` feature sets the panic-handler to reset the device. You can deploy
using the USB bootloader:

```sh
$ make deploy
```

## Dependencies when building and flashing using the sparkfun bootloader

* apt install gcc-arm-none-eabi binutils-arm-none-eabi clang libclang-dev
* cargo install cargo-binutils
* rustup target add thumbv7em-none-eabihf
* rustup component add llvm-tools-preview

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

* lowaccel: increase accel and gyro range to expect greater forces impacted by
    breaking waves.

* raw: store raw data on SD-card (experimental)

* host-tests: used to disable code that doesn't compile on host, for running
    host unit tests. Best used through `make host-test`.

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

# Troubleshooting

1. On Ubuntu 22 the package `brltty` claims the Artemis USB device and the tty
   device disappears, remove it if you don't need it.
