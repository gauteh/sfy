import numpy as np
import xarray as xr
import pandas as pd
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


def spec_stats(ds: xr.Dataset, raw=False, window=(20 * 60)) -> xr.Dataset:
    """
    Return DataSet with Hm0, Tc, Tz, m0, m2, m4 and elevation spectra.
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

    def stat(zz):
        f, P = signal.welch(ds.frequency, zz)
        if not raw:
            _, _, P = signal.imu_cutoff_rabault2022(f, P)
        return *signal.spec_stats(f, P), f, P

    with ThreadPoolExecutor() as x:
        m0, m1, m2, m4, hm0, Tm01, Tm02, f, P = zip(*x.map(stat, z))

    i = np.append(i, len(z))

    time = ds.time[i - N].values

    assert len(hm0) == len(time)

    P = np.vstack(P)

    logger.debug('Building dataset..')

    return xr.Dataset(
        {
            'hm0':
            xr.DataArray(
                np.array(hm0),
                dims=[
                    'time',
                ],
                attrs={
                    'unit':
                    'm',
                    'long_name':
                    'sea_surface_wave_significant_height',
                    'description':
                    'Significant wave height calculated in the frequency domain from the first moment.'
                }),
            'Tm01':
            xr.DataArray(np.array(Tm01),
                         dims=['time'],
                         attrs={
                             'unit': 's',
                             'long_name': 'sea_surface_wave_mean_period',
                             'description': 'Mean zero crossing period (m0/m1)'
                         }),
            'Tm02':
            xr.DataArray(
                np.array(Tm02),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'long_name': 'sea_surface_wave_mean_period',
                    'description': 'Mean zero crossing period (sqrt(m0/m2))'
                }),
            'm0':
            xr.DataArray(
                np.array(m0),
                dims=['time'],
                attrs={'description': 'Zeroth order moment from spectrum'}),
            'm1':
            xr.DataArray(
                np.array(m1),
                dims=['time'],
                attrs={'description': 'First order moment from spectrum'}),
            'm2':
            xr.DataArray(
                np.array(m2),
                dims=['time'],
                attrs={'description': 'Second order moment from spectrum'}),
            'm4':
            xr.DataArray(
                np.array(m4),
                dims=['time'],
                attrs={'description': 'Forth order moment from spectrum'}),
            'E':
            xr.DataArray(
                P,
                dims=['time', 'frequency'],
                attrs={
                    'unit':
                    'm^2/Hz',
                    'long_name':
                    'sea_surface_wave_variance_spectral_density',
                    'description':
                    'Sea surface elevation spectrum (variance density spectrum) calculated using Welch method.',
                }),
        },
        coords={
            'time': time,
            'frequency': np.array(f[0])
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

    d['u_z'] = xr.DataArray(u_z.astype(np.float32),
                            coords=[('time', ds.time.data)],
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

    d['u_x'] = xr.DataArray(u_x.astype(np.float32),
                            coords=[('time', ds.time.data)],
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

    d['u_y'] = xr.DataArray(u_y.astype(np.float32),
                            coords=[('time', ds.time.data)],
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


def unique_positions(ds):
    """
    Remove duplicate positions and NaTs
    """
    _, ui = np.unique(ds.position_time.values, return_index=True)
    ds = ds.isel(position_time=ui)
    ds = ds.isel(position_time=~pd.isna(ds.position_time.values))

    return ds


def estimate_frequency(ds, N=None):
    """
    The actual frequency on the IMU may vary with up to 10% (see https://github.com/gauteh/sfy/issues/125). This function
    estimates the actual frequency from the GPS timestamps.
    """
    if N is None:
        N = ds.attrs.get('package_length', 1024)

    n = len(ds.package_start.values)  # number of packages

    if n < 2:
        logger.warning(
            'less than two packages for estimating frequency, using assumed frequency.'
        )
        return np.array([ds.attrs['frequency']])

    f = []

    for i in range(n - 1):
        t0 = ds.package_start.values[i]
        t1 = ds.package_start.values[i + 1]

        # length of batch including offsets of timestamps.
        m = N - ds.offset.values[i] + ds.offset.values[i + 1]

        ddt = (t1 - t0).astype('timedelta64[ms]').astype(float)
        f.append(float(m) / (ddt / 1000.))

    f.append(f[-1])  # backwards diff for last package.

    return np.array(f)


def groupby_segments(ds, eps_gap=3.):
    """
    Group a dataset on gaps in the data. Cannot split along `received` dimension as well.
    """
    N = ds.attrs.get('package_length', 1024)
    n = len(ds.package_start.values)  # number of packages

    PDT = N / ds.attrs['frequency'] * 1000.  # length of package in ms
    pdt = np.diff(
        ds.package_start.values).astype('timedelta64[ms]').astype(float)

    ip = np.argwhere(np.abs(pdt)
                     > (PDT +
                        eps_gap * 1000.))  # index in package_starts and ends
    ip = np.append(0, ip)
    ip = np.append(ip, n)
    ip = np.unique(ip)

    group = np.zeros(ds.time.shape)
    ipp = ip * N

    for i, (ip0, ip1) in enumerate(zip(ipp[:-1], ipp[1:])):
        group[ip0:ip1] = i

    return ds.groupby(xr.DataArray(group, dims=('time')))


def seltime(ds, start, end):
    """
    Trim dataset to between start and end (both along time and packages)
    """
    pdt = ds.package_start.values.astype('datetime64[ms]').astype(float)
    fstart = pd.to_datetime(start).to_datetime64().astype(
        'datetime64[ms]').astype(float)
    fend = pd.to_datetime(end).to_datetime64().astype('datetime64[ms]').astype(
        float)

    ip0 = np.argmax(pdt >= fstart)
    ip1 = np.argmax(pdt > fend)

    return ds.sel(time=slice(start, end)).isel(package=slice(ip0, ip1))


def splitby_segments(ds, eps_gap=3.):
    """
    Split a dataset on gaps in the data.
    """
    N = ds.attrs.get('package_length', 1024)
    n = len(ds.package_start.values)  # number of packages

    PDT = N / ds.attrs['frequency'] * 1000.  # length of package in ms
    pdt = np.diff(
        ds.package_start.values).astype('timedelta64[ms]').astype(float)

    ip = np.argwhere(np.abs(pdt) > (
        PDT + eps_gap * 1000.)) + 1  # index in package_starts and ends
    ip = np.append(0, ip)
    ip = np.append(ip, n)
    ip = np.unique(ip)

    dss = []

    for ip0, ip1 in zip(ip[:-1], ip[1:]):
        ipp0 = ip0 * N
        ipp1 = ip1 * N

        assert ip1 > ip0

        d = ds.isel(time=slice(ipp0, ipp1)) \
                .isel(package=slice(ip0, ip1))

        dss.append(d)

    return dss


def retime(ds, eps_gap=3.):
    """
    Re-time a dataset based on the estimated frequency and a best fit of timestamps. Assuming the frequency is
    stable throughout the dataset.
    """

    logger.debug('Re-timing dataset based on estimated frequency..')
    N = ds.attrs.get('package_length', 1024)
    n = len(ds.package_start.values)  # number of packages

    assert n * N == len(
        ds.time), "dataset has been sliced in time before retiming"

    PDT = N / ds.attrs['frequency'] * 1000.  # length of package in ms
    pdt = np.diff(
        ds.package_start.values).astype('timedelta64[ms]').astype(float)

    if len(pdt) > 1 and np.max(np.abs(pdt)) >= (PDT + eps_gap * 1000.):
        logger.warning(
            f"Re-timing: gap greater than {eps_gap}s in data, splitting and combining"
        )
        dss = list(map(retime, splitby_segments(ds, eps_gap)))
        logger.info(f'Split dataset into {len(dss)} segments, merging..')
        # ds = xr.concat(dss, dim=('time'), data_vars='minimal')
        # ds = xr.combine_by_coords(dss)
        ds = xr.merge(dss)

        return ds

    fs = np.median(estimate_frequency(ds))

    # Find the best estimate for the start of the dataset based on the timestamps
    on = np.arange(0, n) * N + ds.offset.values
    od = (on * 1000. / fs).astype('timedelta64[ms]')
    t0s = ds.package_start.values - od
    t0 = np.mean(
        t0s.astype('datetime64[ns]').astype(float)).astype('datetime64[ns]')

    tt = np.arange(0, n * N) * 1000. / fs
    t = t0 + tt.astype('timedelta64[ms]')

    if 'fir_adjusted' in ds.attrs:
        t = t + np.timedelta64(int(ds.attrs['fir_adjusted']),
                               ds.attrs['fir_adjusted:unit'])

    assert len(t) == len(ds.w_z)
    assert len(t) == len(ds.time)

    oldtime = ds.time.values

    ds = ds.assign_coords(
        retime=('time', t),
        oldtime=('time', oldtime)).set_index(time='retime').assign_attrs({
            'estimated_frequency':
            fs,
            'estimated_frequency:unit':
            'Hz'
        })

    return ds
