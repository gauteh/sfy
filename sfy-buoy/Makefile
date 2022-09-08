.DEFAULT_GOAL := build
USB ?= /dev/ttyUSB0
T ?= debug

ifeq "$(T)" "release"
    ELF:= target/thumbv7em-none-eabihf/release/sfy-artemis
    override CARGO_FLAGS+=--release
else ifeq "$(T)" "r"
    ELF:= target/thumbv7em-none-eabihf/release/sfy-artemis
    override CARGO_FLAGS+=--release
else
    ELF:= target/thumbv7em-none-eabihf/debug/sfy-artemis
endif

bump-jlink:
	python3 ../tools/bump-jlink

build:
	cd sfy-artemis && cargo build $(CARGO_FLAGS)

bin: build
	arm-none-eabi-objcopy -S -O binary $(ELF) target/sfy-buoy.bin
	# cd sfy-artemis && cargo objcopy $(CARGO_FLAGS) -- -O binary ../target/sfy-buoy.bin

flash: bin
	python3 ../tools/svl/svl.py -f target/sfy-buoy.bin $(USB) -v

jlink-flash: bin bump-jlink
	sh jlink-flash.sh

deploy: bin
	python3 ../tools/svl/svl.py -f target/sfy-buoy.bin $(USB) -v

com:
	picocom -e c -b 115200 $(USB)

defmt-serial:
	socat $(USB),raw,echo=0,b115200 STDOUT | defmt-print -e $(ELF)

gdb-server: bump-jlink
	JLinkGDBServer -select USB -device AMA3B1KK-KBR -endian little -if SWD -noir -noLocalhostOnly -nohalt

gdb-flash:
	gdb-multiarch --command=flash.gdb $(ELF)

gdb-debug:
	gdb-multiarch --command=debug.gdb $(ELF)

defmt-rtt:
	# stdbuf -i0 -e0 -o0 JLinkRTTClient < /dev/null | stdbuf -i0 -e0 -o0 tail -f -n +23 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy
	stdbuf -i0 -e0 -o0 nc localhost 19021 | defmt-print -e $(ELF)

rtt:
	# stdbuf -i0 -e0 -o0 JLinkRTTClient < /dev/null | stdbuf -i0 -e0 -o0 tail -f -n +23 | defmt-print -e target/thumbv7em-none-eabihf/debug/sfy-buoy
	stdbuf -i0 -e0 -o0 nc localhost 19021

host-test:
	cargo test
