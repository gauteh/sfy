from . import *

import time

def test_add_template(serial):
    serial.write('{ "req": "note.template", "file": "axl_test.qo", "body": { "timestamp": 14, "offset": 14 } }\r')
    # serial.write('{"req":"note.template", "file":"axl_test.qo", "body": { "timestamp": 14, "offset": 14 } }\r')
    time.sleep(1)
    r = serial.readline()

    assert 'bytes' in r
    print(r)

def test_add_note_to_template(serial):
    serial.write('{ "req":  "note.add", "file": "axl_test.qo", "body": { "timestamp": 15, "offset":  43 } }\r')
    # serial.write('{"req":"note.add", "file":"axl_test.qo", "body": { "timestamp": 0, "offset": 0 }, "sync": true }\r')
    time.sleep(1)

    r = serial.readline()
    assert 'template":true' in r
    print(r)
