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
    logger.debug(f'Splitting into {len(i)} windows..')
    z = np.split(z, i)

    # Calculate hm0 for each window
    logger.debug(f'Calculating Hm0 for {len(z)} 20 minute segments..')

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
        name='hm0',
        attrs={
            'unit':
            'm',
            'long_name':
            'sea_surface_wave_significant_height',
            'description':
            'Significant wave height calculated in the frequency domain from the first moment.'
        })

def displacement(ds: xr.Dataset, filter_freqs=None):
        logger.info(
            f'Integrating displacment, filter frequencies: {filter_freqs}.')

        u_z = signal.integrate(ds.w_z,
                               ds.dt,
                               order=2,
                               freqs=filter_freqs,
                               method='dft')

        u_x = signal.integrate(ds.w_x,
                               ds.dt,
                               order=2,
                               freqs=filter_freqs,
                               method='dft')
        u_y = signal.integrate(ds.w_y,
                               ds.dt,
                               order=2,
                               freqs=filter_freqs,
                               method='dft')

        d = xr.Dataset()

        d['u_z'] = xr.DataArray(
            u_z.astype(np.float32),
            coords = [('time', ds.time.data)],
            attrs={
                'unit': 'm',
                'long_name': 'sea_water_wave_z_displacement',
                'description':
                'Horizontal z-axis displacement (integrated)',
                'detrended': 'yes',
                'filter': 'butterworth (10th order), two-ways',
                'filter_freqs': filter_freqs,
                'filter_freqs:unit': 'Hz',
            })

        d['u_x'] = xr.DataArray(
            u_x.astype(np.float32),
            coords = [('time', ds.time.data)],
            attrs={
                'unit': 'm',
                'long_name': 'sea_water_wave_x_displacement',
                'description':
                'Horizontal x-axis displacement (integrated)',
                'detrended': 'yes',
                'filter': 'butterworth (10th order), two-ways',
                'filter_freqs': filter_freqs,
                'filter_freqs:unit': 'Hz',
            })

        d['u_y'] = xr.DataArray(
            u_y.astype(np.float32),
            coords = [('time', ds.time.data)],
            attrs={
                'unit': 'm',
                'long_name': 'sea_water_wave_y_displacement',
                'description':
                'Horizontal y-axis displacement (integrated)',
                'detrended': 'yes',
                'filter': 'butterworth (10th order), two-ways',
                'filter_freqs': filter_freqs,
                'filter_freqs:unit': 'Hz',
            })

        return d
