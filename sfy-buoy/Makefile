.DEFAULT_GOAL := build

build:
	cd sfy-artemis && cargo build $(CARGO_FLAGS)

objcopy: build
	arm-none-eabi-objcopy -S -O binary target/thumbv7em-none-eabihf/debug/sfy-artemis target/sfy-buoy.bin
	# cd sfy-artemis && cargo objcopy $(CARGO_FLAGS) -- -O binary ../target/sfy-buoy.bin

flash: objcopy
	python3 ../tools/svl/svl.py -f target/sfy-buoy.bin /dev/ttyUSB0 -v

jlink-flash: objcopy
	sh jlink-flash.sh

deploy:
	cd sfy-artemis && DEFMT_LOG=warn,sfy=debug,sfy::note=trace cargo objcopy --features deploy $(CARGO_FLAGS) -- -O binary target/sfy-buoy.bin
	python3 ../tools/svl/svl.py -f target/sfy-buoy.bin /dev/ttyUSB0 -v

com:
	picocom -e c -b 115200 /dev/ttyUSB0

defmt-serial:
	socat /dev/ttyUSB0,raw,echo=0 STDOUT | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-artemis

gdb-server:
	JLinkGDBServer -select USB -device AMA3B1KK-KBR -endian little -if SWD -noir -noLocalhostOnly -nohalt

gdb-flash:
	gdb-multiarch --command=flash.gdb target/thumbv7em-none-eabihf/debug/sfy-artemis

gdb-debug:
	gdb-multiarch --command=debug.gdb target/thumbv7em-none-eabihf/debug/sfy-artemis

defmt-rtt:
	# stdbuf -i0 -e0 -o0 JLinkRTTClient < /dev/null | stdbuf -i0 -e0 -o0 tail -f -n +23 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy
	stdbuf -i0 -e0 -o0 nc localhost 19021 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-artemis

rtt:
	# stdbuf -i0 -e0 -o0 JLinkRTTClient < /dev/null | stdbuf -i0 -e0 -o0 tail -f -n +23 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy
	stdbuf -i0 -e0 -o0 nc localhost 19021

host-test:
	cargo test --features storage
