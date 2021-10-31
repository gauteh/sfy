#! python

import serial
import io
import time

print('opening serial..')
with serial.Serial('/dev/ttyACM0', timeout = .5) as ser:
    sio = io.TextIOWrapper(io.BufferedRWPair(ser, ser), line_buffering=True, newline = '\r\n')
    # sio = io.TextIOWrapper(ser, line_buffering=False, newline = '\r')

    print('get status..')
    sio.write('{"req":"card.status"}\r')
    print(f'status: {sio.readline()}')

    print('registering..')
    sio.write('{"req":"hub.set", "product": "com.vetsj.gaute.eg:sby", "sn":"cain"}\r')
    print(sio.readline())

    print('syncing..')
    sio.write('{"req":"hub.sync"}\r')
    print(sio.readline())


    print('add some note file..')
    sio.write('{"req":"note.add", "file":"test.db", "note": "?", "body": { "temp": 374.3 }, "sync": true }\r')
    print(sio.readline())

    while True:
        print("hub sync status:")
        sio.write('{"req":"hub.sync.status"}\r')
        print(sio.readline())
        print()

        time.sleep(5)

