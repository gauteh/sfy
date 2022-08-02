#!/usr/bin/env bash

cd target/

JLinkExe -device AMA3B1KK-KBR -autoconnect 1 -if swd -speed 4000 <<EOF
h
loadbin sfy-buoy.bin 0x10000
r
go
EOF
