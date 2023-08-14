import pytest
import numpy as np
from datetime import datetime, timezone
import shutil

from sfy import hub
from sfy.axl import AxlCollection
from . import *

@pytest.mark.skipif(shutil.which('sfypack') is None, reason = 'sfypack not installed')
def test_parse_collection():
    f = '../sfy-buoy/tests/data/44.5'
    c = AxlCollection.from_storage_file('WAVEBUG04', '778', f)
    print(c)

@pytest.mark.xfail()
@pytest.mark.skipif(shutil.which('sfypack') is None, reason = 'sfypack not installed')
def test_raw_accel_gyro():
    f = '../sfy-buoy/tests/data/32.5'
    c = AxlCollection.from_storage_file('WAVEBUG23', 'XXX', f, raw=True)
    print(c)
