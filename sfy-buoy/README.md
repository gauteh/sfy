# sfy-buoy (small friendly buoy)

## Building for deployment

```sh
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

* continuous: transmits data continuously, at the cost of power.

* deploy: turns on `asm::wfi` in main loop over busy wait.

* storage: WIP: store data on SD card.

* host-tests: used to disable code that doesn't compile on host, for running
    host unit tests. Best used though `make host-test`.

### Environment variables

* BUOYSN: the name of the buoy as it appears on the data server.

* BUOYPR: the product name used for the modem. determines which account the data
    is sent to on notehub.io.

* DEFMT_LOG: defmt log levels, leave empty to compile out.
