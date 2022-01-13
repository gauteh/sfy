import pytest
from . import *

import time

@pytest.mark.notecard
def test_get_status(serial):
    print('get status..')
    serial.write('{"req":"card.status"}\r')
    print(f'status: {serial.readline()}')

@pytest.mark.notecard
def test_add_note(serial):
    print('add some note file..')
    serial.write('{"req":"note.add", "file":"test.db", "note": "?", "body": { "temp": 374.3 }, "sync": true }\r')
    print(serial.readline())

    for _ in range(0,10):
        print("hub sync status:")
        serial.write('{"req":"hub.sync.status"}\r')
        r = serial.readline()
        print(r)

        if 'completed' in r:
            break

        time.sleep(5)

@pytest.mark.notecard
def test_hub_sync_status(serial):
    serial.write('{"req":"hub.sync.status"}\r')
    r = serial.readline()
    print(r)

@pytest.mark.notecard
def test_hub_do_sync(serial):
    serial.write('{"req":"hub.sync"}\r')
    print(serial.readline())

    for _ in range(0,10):
        print("hub sync status:")
        serial.write('{"req":"hub.sync.status"}\r')
        r = serial.readline()
        print(r)

        if 'completed' in r:
            break

        time.sleep(5)
