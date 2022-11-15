import pytest
import numpy as np
from datetime import datetime, timezone

from sfy import hub
from sfy.axl import AxlCollection
from . import *

@needs_hub
def test_collect(sfyhub):
    b = sfyhub.buoy("dev864475044204278")
    pcks = b.axl_packages_range(
        datetime(2022, 4, 26, 11, 34, tzinfo=timezone.utc),
        datetime(2022, 4, 26, 11, 35, tzinfo=timezone.utc))
    assert len(pcks) > 2

    c = AxlCollection(pcks)
    print("duration:", c.duration)
    print(f"len= {len(pcks)}")

    np.testing.assert_almost_equal(c.duration, len(pcks) * 1024 / 52.)


@needs_hub
def test_segment(sfyhub):
    b = sfyhub.buoy("dev864475044204278")
    pcks = b.axl_packages_range(
        datetime(2022, 4, 26, 11, 34, tzinfo=timezone.utc),
        datetime(2022, 4, 26, 11, 35, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    segments = list(c.segments())
    assert len(segments) == 2
    assert sum((len(s) for s in segments)) == len(c)

@needs_hub
def test_join_collections(sfyhub):
    b = sfyhub.buoy("dev864475044204278")
    pcks = b.axl_packages_range(
        datetime(2022, 4, 26, 11, 34, tzinfo=timezone.utc),
        datetime(2022, 4, 26, 11, 35, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    segments = list(c.segments())
    assert len(segments) == 2
    assert sum((len(s) for s in segments)) == len(c)

    jc = segments[0] + segments[1]
    assert len(jc) == len(c)

    # Duplicates are removed
    jc = segments[0] + segments[1] + segments[0]
    assert len(jc) == len(c)
