.DEFAULT_GOAL := build

build:
	cargo build --release

objcopy: build
	cargo objcopy --release -- -O binary target/sby-buoy.bin

flash: objcopy
	python3 ../tools/svl/svl.py -f target/sby-buoy.bin /dev/ttyUSB0 -v

deploy:
	DEFMT_LOG=warn,sfy=debug,sfy::note=trace cargo objcopy --features deploy -- -O binary target/sby-buoy.bin
	python3 ../tools/svl/svl.py -f target/sby-buoy.bin /dev/ttyUSB0 -v

com:
	picocom -e c -b 115200 /dev/ttyUSB0

defmt:
	cat /dev/ttyUSB0 > /dev/null
	tail -f /dev/ttyUSB0 | defmt-print -e target/thumbv7em-none-eabihf/release/sfy-buoy

gdb-server:
	JLinkGDBServer -select USB -device AMA3B1KK-KBR -endian little -if SWD -noir -noLocalhostOnly

gdb-flash:
	gdb-multiarch --command=flash.gdb target/thumbv7em-none-eabihf/release/sfy

gdb-debug:
	gdb-multiarch --command=debug.gdb target/thumbv7em-none-eabihf/debug/sfy

defmt-rtt:
	# stdbuf -i0 -e0 -o0 JLinkRTTClient < /dev/null | stdbuf -i0 -e0 -o0 tail -f -n +23 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy
	stdbuf -i0 -e0 -o0 nc localhost 19021 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy

rtt:
	# stdbuf -i0 -e0 -o0 JLinkRTTClient < /dev/null | stdbuf -i0 -e0 -o0 tail -f -n +23 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy
	stdbuf -i0 -e0 -o0 nc localhost 19021

host-test:
	$(eval TARGET = $(shell rustc -vV | sed -n 's|host: ||p'))
	cargo test --features host-tests --target=$(TARGET)
