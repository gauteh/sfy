import pytest
import serial as ser
import io

@pytest.fixture
def serial():
    print('opening serial..')
    serial = ser.Serial('/dev/ttyACM0', timeout = .5)
    sio = io.TextIOWrapper(io.BufferedRWPair(serial, serial), line_buffering=True, newline = '\r\n')
    return sio

