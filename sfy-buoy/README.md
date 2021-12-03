# sfy-buoy (small friendly buoy)

## Dependencies when building and flashing using the sparkfun bootloader

* apt install binutils-arm-none-eabi
* cargo install cargo-binutils
* rustup component add llvm-tools-preview

## Filesystem

- FAT32+SD: https://github.com/rust-embedded-community/embedded-sdmmc-rs | https://github.com/Spxg/fat32
- Littlefs
- [tickv](https://github.com/tock/tock/tree/master/libraries/tickv)
