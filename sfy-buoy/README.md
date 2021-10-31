# sfy-buoy (small friendly buoy)

## Dev dependencies

* apt install binutils-arm-none-eabi
* cargo install cargo-binutils
* rustup component add llvm-tools-preview

## Uploader

* https://github.com/sparkfun/Apollo3_Uploader_SVL

linker script/memory.x from: https://github.com/sparkfun/Apollo3_Uploader_SVL/blob/main/0x10000.ld

## Resources

* https://github.com/rust-embedded/cortex-m-quickstart

# OS

## Filesystem

- FAT32+SD: https://github.com/rust-embedded-community/embedded-sdmmc-rs
  * https://github.com/Spxg/fat32
- Littlefs
- [tickv](https://github.com/tock/tock/tree/master/libraries/tickv)
