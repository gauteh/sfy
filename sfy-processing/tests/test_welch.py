import numpy as np
import scipy as sc
from sfy import axl, signal
# from sfy.axl import AxlCollection
from sfy.spec import SpecCollection
import matplotlib.pyplot as plt
from datetime import datetime, timezone
from . import *


def test_nseg_20min():
    n = 1
    NSEG = 4096
    NOVERLAP = NSEG // 2
    fs = 52.

    while True:
        N = n - 1
        N = NSEG + (NSEG - NOVERLAP) * N

        duration = N / fs

        print("duration: ", duration)

        if duration >= 20. * 60.:
            print("segments required for above 20 minutes:", n)
            break

        n += 1

@needs_hub
def test_collect(sfyhub):
    b = sfyhub.buoy("dev860264054655247")
    pcks = b.spec_packages_range(
        datetime(2025, 5, 6, 15, 00, tzinfo=timezone.utc),
        datetime(2025, 5, 7, 12, 00, tzinfo=timezone.utc))
    assert len(pcks) > 10

    c = SpecCollection(pcks)
    print("duration:", c.duration)
    print(f"len= {len(pcks)}")

    assert c.start == c.time[0]

    np.testing.assert_almost_equal(c.duration, len(pcks) * 1024 / 52.)
