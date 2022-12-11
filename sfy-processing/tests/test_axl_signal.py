import numpy as np
import scipy as sc
from datetime import datetime, timezone
from sfy import axl, signal
from pytest import approx

from . import *


@needs_hub
def test_axl_v5_quiet(sfyhub):
    b = sfyhub.buoy("dev867648043595907")
    pcks = b.axl_packages_range(
        datetime(2022, 12, 11, 15, 00, tzinfo=timezone.utc),
        datetime(2022, 12, 11, 15, 30, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()

    assert len(ds.time) / ds.frequency > (20 * 60)

    # this is a period where the buoy was resting quietly
    print(np.mean(ds.w_z))
    print(np.std(ds.w_z))
    print(np.max(np.abs(np.mean(ds.w_z)-ds.w_z)))
    assert np.mean(ds.w_z) == approx(9.8, abs=0.2)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.33)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)

@needs_hub
def test_axl_v3_quiet(sfyhub):
    b = sfyhub.buoy("dev867648043598489")
    pcks = b.axl_packages_range(
        datetime(2022, 11, 30, 11, 0, tzinfo=timezone.utc),
        datetime(2022, 11, 30, 11, 25, tzinfo=timezone.utc))

    c = axl.AxlCollection(pcks)
    ds = c.to_dataset()

    # this is a period where the buoy was resting quietly
    assert np.mean(ds.w_z) == approx(9.8, abs=0.3)
    assert np.mean(ds.w_x) == approx(0.0, abs=0.2)
    assert np.mean(ds.w_y) == approx(0.0, abs=0.2)
