import pytest
import numpy as np
from datetime import datetime, timezone

from sfy import hub
from sfy.axl import AxlCollection
from . import sfyhub

def test_parse_collection():
    f = '../sfy-buoy/tests/data/74.1'
    c = AxlCollection.from_storage_file('WAVEBUG04', '778', f)
    print(c)

