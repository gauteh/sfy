import numpy as np
from datetime import datetime, timezone
from sfy import axl
from . import sfyhub


def test_position_time_wrong(sfyhub):
    # sfydata axl ts bug04 --tx-start 2022-07-06 --tx-end 2022-07-20 --start 2022-07-06T09:00:00 --end 2022-07-07T07:55:00 --file bug04_xx.nc
    tx_start = datetime(2022, 7, 6, tzinfo=timezone.utc)
    tx_end = datetime(2022, 7, 20, tzinfo=timezone.utc)
    start = datetime(2022, 7, 6, 9, tzinfo=timezone.utc)
    end = datetime(2022, 7, 7, 7, 55, tzinfo=timezone.utc)

    buoy = sfyhub.buoy('bug04')
    pcks = buoy.axl_packages_range(tx_start, tx_end)
    pcks = axl.AxlCollection(pcks)
    pcks.clip(start, end)

    segments = list(pcks.segments())
    assert len(segments) == 2
    print(segments)

    # timestamps
    ds = pcks.to_dataset()
    print(ds.position_time[:])
    print(ds.position_time[np.isfinite(ds.position_time)])

    assert all(ds.position_time[np.isfinite(ds.position_time)] > np.datetime64(int(start.timestamp()), 's'))


