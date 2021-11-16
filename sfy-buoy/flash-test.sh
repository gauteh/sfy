#! /bin/bash

echo flashing $1

tmux split-window "nc localhost 19021 | defmt-print -e ${1}"

gdb-multiarch -x flash.gdb $1


