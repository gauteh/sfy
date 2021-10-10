.DEFAULT_GOAL := build

build:
	cargo build --release

objcopy: build
	cargo objcopy --release -- -O binary target/sby-buoy.bin

flash: objcopy
	python3 ../tools/svl/svl.py -f target/sby-buoy.bin /dev/ttyUSB0 -v

com:
	picocom -e c -b 115200 /dev/ttyUSB0
