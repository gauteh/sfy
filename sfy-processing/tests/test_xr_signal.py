import numpy as np
import pandas as pd
from datetime import datetime, timezone
from pytest import approx
import matplotlib.pyplot as plt
import xarray as xr

import sfy.xr
from sfy.axl import AxlCollection
from . import *

@needs_hub
def test_xr_spec_stats(sfyhub):
    b = sfyhub.buoy("dev867648043576717")
    pcks = b.axl_packages_range(
        datetime(2023, 4, 20, 9, 8, tzinfo=timezone.utc),
        datetime(2023, 4, 20, 9, 38, tzinfo=timezone.utc))
    c = AxlCollection(pcks)

    ds = c.to_dataset()
    st = sfy.xr.spec_stats(ds)
    print(st)

