# Registers notecard in the project

import sys
import serial
import time
import io

project = 'no.met.gauteh:sfy'
buoy = 'cain'

if len(sys.argv) > 1:
    project = sys.argv[1]

if len(sys.argv) > 2:
    project = sys.argv[2]

print(f"{project=}")
print(f"{buoy=}")

print('opening serial..')
ser = serial.Serial('/dev/ttyACM0', timeout = .5)
sio = io.TextIOWrapper(io.BufferedRWPair(ser, ser), line_buffering=True, newline = '\r\n')


sio.write('{"req":"hub.set", "product": "%s", "sn":"%s"}\r' % (project, buoy))

print("syncing..")
sio.write('{"req":"hub.sync", "allow": true}\r')
print(sio.readline())

for _ in range(0,10):
    print("hub sync status:")
    sio.write('{"req":"hub.sync.status"}\r')
    r = sio.readline()
    print(r)

    print("hub status:")
    sio.write('{"req":"hub.status"}\r')
    r = sio.readline()
    print(r)

    print("card wireless:")
    sio.write('{"req":"card.wireless"}\r')
    r = sio.readline()
    print(r)

    if 'completed' in r:
        break

    time.sleep(5)
