#!/usr/bin/env bash

set -epx

SN=${BUOYSN}
N0=$1
N1=$2
OUT=$3

echo "SN: ${SN}"
echo "Building binaries in range ${N0} to ${N1}, output dir: ${OUT}"

for n in $(seq ${N0} ${N1}); do
  n=$(printf "%03d" ${n})
  NSN="${SN}${n}"
  BUOYSN=${NSN} GPS_PERIOD=${GPS_PERIOD} GPS_HEARTBEAT=${GPS_HEARTBEAT} SYNC_PERIOD=${SYNC_PERIOD} make T=r bin
  mv target/sfy-buoy.bin "${OUT}/${NSN}.bin"
done
