.DEFAULT_GOAL := build

build:
	cargo build --release

objcopy: build
	cargo objcopy --release -- -O binary target/sby-buoy.bin

flash: objcopy
	python3 ../tools/svl/svl.py -f target/sby-buoy.bin /dev/ttyUSB0 -v

com:
	picocom -e c -b 115200 /dev/ttyUSB0

defmt:
	cat /dev/ttyUSB0 > /dev/null
	tail -f /dev/ttyUSB0 | defmt-print -e target/thumbv7em-none-eabihf/release/sby-buoy
