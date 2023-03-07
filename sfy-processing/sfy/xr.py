import numpy as np
import xarray as xr
import logging
from concurrent.futures import ThreadPoolExecutor
from . import signal

logger = logging.getLogger(__name__)


def hm0(ds: xr.Dataset, raw=False, window=(20 * 60)) -> xr.DataArray:
    """
    Return DataArray with Hm0.
    """

    z = ds.w_z.values

    # split into windows
    N = int(window * ds.frequency)
    N = min(N, len(z))
    i = np.arange(N, len(z), N).astype(np.int32)
    z = np.split(z, i)

    # Calculate hm0 for each window
    logger.debug('Calculating Hm0 for all 20 minute segments..')

    def hm0(zz):
        f, P = signal.welch(ds.frequency, zz)
        if not raw:
            _, _, P = signal.imu_cutoff_rabault2022(f, P)
        return signal.hm0(f, P)

    with ThreadPoolExecutor() as x:
        hm0 = list(x.map(hm0, z))

    i = np.append(i, len(z))

    time = ds.time[i - N].values

    logger.debug('Building dataarray..')
    return xr.DataArray(
        hm0,
        coords=[('time', time)],
        attrs={
            'unit':
            'm',
            'long_name':
            'sea_surface_wave_significant_height',
            'description':
            'Significant wave height calculated in the frequency domain from the first moment.'
        })
