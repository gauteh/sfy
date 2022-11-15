import pytest
import numpy as np
from datetime import datetime, timezone
import shutil

from sfy import hub
from sfy.axl import AxlCollection
from . import *

@pytest.mark.skipif(shutil.which('sfypack') is None, reason = 'sfypack not installed')
def test_parse_collection():
    f = '../sfy-buoy/tests/data/74.1'
    c = AxlCollection.from_storage_file('WAVEBUG04', '778', f)
    print(c)

