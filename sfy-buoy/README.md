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

## Feature flags and environment variables

### Features

* defmt-serial: logs defmt-messages over serial rather than RTT. So that you can
    read messages without a hardware-debugger. See `make defmt-serial`.

* continuous: transmits data continuously, at the cost of power.

* 20hz: set output sample rate of waves to 20Hz, rather than 52hz.

* deploy: turns on `asm::wfi` in main loop over busy wait.

* storage: WIP: store data on SD card.

* host-tests: used to disable code that doesn't compile on host, for running
    host unit tests. Best used though `make host-test`.

### Environment variables

* BUOYSN: the name of the buoy as it appears on the data server.

* BUOYPR: the product name used for the modem. determines which account the data
    is sent to on notehub.io.

* GPS_PERIOD: Sample interval for GPS.

* DEFMT_LOG: defmt log levels, leave empty to compile out.

# Troubleshooting

1. On Ubuntu 22 the package `brltty` claims the Artemis USB device and the tty
   device disappears, remove it if you don't need it.

