# This connects to the GDB server running locally.
#   - for openocd, use port :3333
#   - for JLinkGDBServer, use port :2331
target extended-remote :2331
# target remote :1337

# Due to https://github.com/rust-embedded/cortex-m-rt/issues/139,
#   we will get an infinite backtrace on a panic!(). Set a finite
#   limit to the backtrace to prevent the debugger falling into
#   an endless loop trying to read the backtrace
set backtrace limit 32

# Load the specified firmware onto the device
load

# Reset the target device before running (using JLinkGDBServer)
# monitor reset

# Reset the target device before running (using openocd)
# monitor reset halt
# b main

# b rust_begin_unwind
# b HardFault

source ../defmt-rtt-gdb.py
defmt-rtt

# shell sleep 2
# Begin running the program
# continue

# set confirm off
# run

