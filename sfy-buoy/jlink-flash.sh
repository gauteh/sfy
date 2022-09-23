#!/usr/bin/env bash

D=$(dirname $1)
B=$(basename $1)
cd ${D}

echo "Flashing: ${B} from ${D}"

JLinkExe -device AMA3B1KK-KBR -autoconnect 1 -if swd -speed 4000 <<EOF
h
loadbin ${B} 0x10000
r
go
EOF
