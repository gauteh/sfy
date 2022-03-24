# sfy-buoy (small friendly buoy)

## Building for deployment

```sh
$ BUOYSN=WAVEBUGXX DEFMT_LOG=debug cargo build --release --features deploy
```

the `deploy` feature sets the panic-handler to reset the device. You can deploy
using the USB bootloader:

```sh
$ make deploy
```

## Dependencies when building and flashing using the sparkfun bootloader

* apt install gcc-arm-none-eabi binutils-arm-none-eabi libclang-common-6.0-dev clang-6.0 libclang-dev
* cargo install cargo-binutils
* rustup target add thumbv7em-none-eabihf
* rustup component add llvm-tools-preview

