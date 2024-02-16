import numpy as np
import xarray as xr
import pandas as pd
import logging
from concurrent.futures import ThreadPoolExecutor
from . import signal

logger = logging.getLogger(__name__)


def welch(ds: xr.Dataset):
    return signal.welch(ds.estimated_frequency, ds.w_z)


def hm0(ds: xr.Dataset, raw=False, window=(20 * 60)) -> xr.DataArray:
    """
    Return DataArray with Hm0.

    Args:

        window: window size to calculate hm0 for (seconds, default 20 minutes).
    """

    return spec_stats(ds, raw, window)['hm0']


def spec_stats(ds: xr.Dataset,
               raw=False,
               window=(20 * 60),
               nperseg=4096,
               order=2) -> xr.Dataset:
    """
    Return Dataset with Hm0, Tc, Tz, m0, m2, m4 and elevation spectra.

    Args:

        window: window size to calculate hm0 for (seconds, default 20 minutes).
    """

    zz = ds.w_z.values
    yy = ds.w_y.values
    xx = ds.w_x.values
    freq = ds.attrs.get('estimated_frequency', ds.attrs['frequency'])

    assert len(zz) == len(yy)
    assert len(xx) == len(yy)

    # The windows need to be the full size, otherwise the statistics will be invalid.

    # split into windows
    N = int(window * freq)

    if len(zz) < N:
        logger.error(
            f'Dataset is shorter {len(zz)/freq}({len(zz)}) than requested window: {window}({N})'
        )
        raise ValueError(
            f'Dataset is shorter {len(zz)/freq}({len(zz)}) than requested window: {window}({N})'
        )

    N = min(N, len(zz))

    i = np.arange(N, len(zz), N).astype(np.int32)
    logger.debug(f'Splitting into {len(i)} windows..')
    z = np.split(zz, i)
    z[-1] = zz[-N:]  # make sure last window is also full length

    y = np.split(yy, i)
    y[-1] = yy[-N:]  # make sure last window is also full length

    x = np.split(xx, i)
    x[-1] = xx[-N:]  # make sure last window is also full length

    if len(ds.w_z.values) <= N:
        assert len(z) == 1, "expected only one window"
    else:
        Ns = [len(zz) for zz in z]
        assert all(
            (ns == N for ns in Ns
             )), f'All windows should be {N} samples length: {Ns=}, {N=}'

    # Calculate stats for each window
    logger.debug(
        f'Calculating spectral stats for {len(z)} 20 minute segments..')

    def stat(zz, yy, xx):
        if np.any(np.isnan(zz)):
            logger.warning(f'NaN values in signal, spectra is set to NaN')
            a = np.full((4096 // 2 + 1, ), np.nan)
            return np.nan, np.nan, np.nan, np.nan, np.nan, np.nan, np.nan, np.nan, np.nan, np.nan, a, a, a, a

        f, Pz = signal.welch(freq, zz, nperseg, order)
        f, Py = signal.welch(freq, yy, nperseg, order)
        f, Px = signal.welch(freq, xx, nperseg, order)
        if not raw:
            i0, _, Pz = signal.imu_cutoff_rabault2022(f, Pz)
            Py[:i0] = 0
            Px[:i0] = 0
        return *signal.spec_stats(f, Pz), f, Pz, Py, Px

    with ThreadPoolExecutor() as ex:
        m_1, m0, m1, m2, m4, hm0, Tm01, Tm02, Tm_10, Tp, f, Pz, Py, Px = zip(
            *ex.map(stat, z, y, x))

    i = np.append(i, len(zz) - 1)  # Add timestamp for last window as well.
    time = ds.time[i].values  # Use timestamp from last sample in each window.

    assert len(hm0) == len(time)

    assert np.isfinite(f[0][0]), "coordinate frequencies all nan"

    Pz = np.vstack(Pz)
    Py = np.vstack(Py)
    Px = np.vstack(Px)

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
                    'Significant wave height calculated in the frequency domain from the zeroth moment (4 * sqrt(m0)).'
                }),
            'Tm01':
            xr.DataArray(
                np.array(Tm01),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'long_name':
                    'sea_surface_wave_mean_period_from_variance_spectral_density_first_frequency_moment',
                    'description': 'First wave period (m0/m1)'
                }),
            'Tm02':
            xr.DataArray(
                np.array(Tm02),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'long_name':
                    'sea_surface_wave_mean_period_from_variance_spectral_density_second_frequency_moment',
                    'description': 'Second wave period (sqrt(m0/m2))'
                }),
            'Tm_10':
            xr.DataArray(
                np.array(Tm_10),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'long_name':
                    'sea_surface_wave_mean_period_from_variance_spectral_density_inverse_frequency_moment',
                    'description': 'Inverse wave period ((m-1/m0))'
                }),
            'Tp':
            xr.DataArray(
                np.array(Tp),
                dims=['time'],
                attrs={
                    'unit':
                    's',
                    'long_name':
                    'sea_surface_wave_period_at_variance_spectral_density_maximum',
                    'description':
                    'Peak period (period with maximum elevation energy)'
                }),
            'm_1':
            xr.DataArray(
                np.array(m_1),
                dims=['time'],
                attrs={'description': 'Inverse order moment from spectrum'}),
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
                Pz,
                dims=['time', 'frequency'],
                attrs={
                    'unit':
                    'm^2/Hz',
                    'long_name':
                    'sea_surface_wave_variance_spectral_density',
                    'description':
                    'Sea surface elevation spectrum (variance density spectrum) calculated using Welch method.',
                }),
            'Ey':
            xr.DataArray(
                Py,
                dims=['time', 'frequency'],
                attrs={
                    'unit':
                    'm^2/Hz',
                    'long_name':
                    'sea_surface_wave_variance_spectral_density',
                    'description':
                    'First horizontal component of sea surface elevation spectrum (variance density spectrum) calculated using Welch method.',
                }),
            'Ex':
            xr.DataArray(
                Px,
                dims=['time', 'frequency'],
                attrs={
                    'unit':
                    'm^2/Hz',
                    'description':
                    'Second horizontal component of sea surface elevation spectrum (variance density spectrum) calculated using Welch method.',
                }),
        },
        coords={
            'time': time,
            'frequency': np.array(f[0])
        })


def displacement(ds: xr.Dataset, filter_freqs=None):
    logger.info(
        f'Integrating displacment, filter frequencies: {filter_freqs}.')

    # sea-surface displacement should point upwards:
    u_z = -signal.integrate(
        ds.w_z, ds.dt, order=2, freqs=filter_freqs, method='dft')

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
        coords=[('time', ds.time.data)],
        attrs={
            'unit': 'm',
            'long_name': 'sea_water_wave_z_displacement',
            'description':
            'Vertical z-axis displacement (integrated) (direction: up)',
            'direction': 'up',
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

    d = d.assign_attrs(ds.attrs)

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

    iFs = ds.attrs['frequency']  # ideal frequency

    for i in range(n - 1):
        t0 = ds.package_start.values[i]
        t1 = ds.package_start.values[i + 1]

        # length of batch including offsets of timestamps.
        m = N - ds.offset.values[i] + ds.offset.values[i + 1]

        ddt = (t1 - t0).astype('timedelta64[ms]').astype(float)

        assert ddt > 0
        ffs = float(m) / (ddt / 1000.)
        if (ffs > (iFs * 1.1)) or (ffs < (iFs * 0.9)):
            logger.warning(f'More than 10% frequency deviation: {ffs}, skipping')
        else:
            f.append(ffs)

    if len(f) > 0:
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

    tdt = ds.time.values.astype('datetime64[ms]').astype(float)
    it0 = np.argmax(tdt >= fstart)
    it1 = np.argmax(tdt > fend)
    # print(ip0, ip1)

    assert end <= ds.time.values[-1]
    assert start >= ds.time.values[0]

    # assert ds.time.dt.is_monotonic_increasing
    # print(pd.to_datetime(ds.time.values).is_monotonic_increasing)

    return ds.isel(time=slice(it0, it1)).isel(package=slice(ip0, ip1))


def splitby_segments(ds, eps_gap=3.) -> list[xr.Dataset]:
    """
    Split a dataset on gaps in the data.
    """
    N = ds.attrs.get('package_length', 1024)
    n = len(ds.package_start.values)  # number of packages

    PDT = N / ds.attrs.get(
        'estimated_frequency',
        ds.attrs.get('frequency')) * 1000.  # length of package in ms
    pdt = np.diff(
        ds.package_start.values).astype('timedelta64[ms]').astype(float)

    ip = np.argwhere(np.abs(pdt) > (
        PDT + eps_gap * 1000.)) + 1  # index in package_starts and ends
    ip = np.append(0, ip)
    ip = np.append(ip, n)
    ip = np.unique(ip)

    assert N * n == ds.dims[
        'time'], "this dataset does not have time and packages corresponding anymore. try splitby_time"

    dss = []

    for ip0, ip1 in zip(ip[:-1], ip[1:]):
        ipp0 = ip0 * N
        ipp1 = ip1 * N

        assert ip1 > ip0

        d = ds.isel(time=slice(ipp0, ipp1)) \
                .isel(package=slice(ip0, ip1))

        assert d.dims['time'] > 0

        dss.append(d)

    return dss


def splitby_time(ds: xr.Dataset, eps_gap=3.) -> list[xr.Dataset]:
    dt = np.diff(ds.time.values).astype('timedelta64[ms]').astype(float)
    ip = np.argwhere(np.abs(dt) > eps_gap * 1000.) + 1
    ip = np.append(0, ip)
    ip = np.append(ip, ds.dims['time'])
    ip = np.unique(ip)

    dss = []

    for ipp0, ipp1 in zip(ip[:-1], ip[1:]):

        assert ipp1 > ipp0

        # find closest package
        ip0 = np.argmin(np.abs(ds.package_start.values - ds.time.values[ipp0]))
        ip1 = np.argmin(
            np.abs(ds.package_start.values - ds.time.values[ipp1 - 1]))

        d = ds.isel(time=slice(ipp0, ipp1)).isel(package=slice(ip0, ip1))

        assert d.dims['time'] > 0

        dss.append(d)

    return dss


def concat(dss):
    """
    Concatenate multiple datasets in a more optimal way than xarray does.

    > Duplicate time and package samples are removed.
    """

    dss = sorted(dss, key=lambda ds: ds.time.values[0])

    # build coordinates
    time = np.concatenate([ds.time.values for ds in dss])
    package = np.concatenate([ds.package.values for ds in dss])

    # concat variables
    cds = xr.Dataset(coords={
        'time': time,
        'package': package
    },
                     attrs=dss[0].attrs)

    for v in dss[0].data_vars:
        if 'time' in dss[0][v].dims:
            values = np.full(time.shape, np.nan, dtype=dss[0][v].dtype)
            offset = 0
            for ds in dss:
                values[offset:offset + len(ds[v])] = ds[v].values
                offset += len(ds[v])
            cds[v] = xr.DataArray(name=v,
                                  data=values,
                                  dims=('time'),
                                  attrs=dss[0][v].attrs)

    for v in dss[0].data_vars:
        if 'package' in dss[0][v].dims:
            values = np.full(package.shape, np.nan, dtype=dss[0][v].dtype)
            offset = 0
            for ds in dss:
                if ds.dims['package'] > 0:
                    values[offset:offset + len(ds[v])] = ds[v].values
                    offset += len(ds[v])
            cds[v] = xr.DataArray(name=v,
                                  data=values,
                                  dims=('package'),
                                  attrs=dss[0][v].attrs)

    # Remove duplicate times - might cause trouble with packages.
    _, ui = np.unique(cds.time.values, return_index=True)
    cds = cds.isel(time=ui)

    _, ui = np.unique(cds.package.values, return_index=True)
    cds = cds.isel(package=ui)

    return cds


def open_mfdataset(path):
    """
    Open multiple sfy datasets and concat (in a more optimized way than xarray does).
    """
    if isinstance(path, str):
        import glob
        path = glob.glob(path)

    return concat([xr.open_dataset(p) for p in path])


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
        # ds = xr.merge(dss)
        ds = concat(dss)

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


def retime_individual(ds):
    """
    Re-time, but only within each package. This better preserves the time, but will cause overlapping samples in case of
    negative time jumps.
    """
    fs = np.median(estimate_frequency(ds))
    logger.info(f'Re-timing dataset with frequency: {fs:.3} Hz.')

    N = ds.attrs.get('package_length', 1024)
    tp = pd.to_timedelta(np.arange(0, N) / fs * 1000., 'ms')

    t = np.full(ds.time.shape, np.nan, dtype='datetime64[ns]')
    for ip in range(len(ds.package_start)):
        t0 = ds.time.values[ip * N]
        ttp = t0 + tp
        t[ip * N:(ip+1)*N] = ttp

    assert len(t) == len(ds.w_z)
    assert len(t) == len(ds.time)
    assert np.all(~np.isnan(t))

    oldtime = ds.time.values

    if np.any(np.diff(t).astype(float) <= 0):
        logger.error('Some samples have overlapping timestamps.')

    tu = np.unique(t)
    if len(tu) != len(t):
        logger.error(f'Non-unique timestamps: {len(t) - len(tu)}')

    if not pd.to_datetime(t).is_monotonic_increasing:
        logger.error('Time is not monotonic increasing.')

    ds = ds.assign_coords(
        retime=('time', t),
        oldtime=('time', oldtime)).set_index(time='retime').assign_attrs({
            'estimated_frequency':
            fs,
            'estimated_frequency:unit':
            'Hz'
        })

    return ds


def fill_gaps(ds: xr.Dataset, fill_value=np.nan, eps_gap=3.) -> xr.Dataset:
    """
    Fill gaps with `fill_value` (default: nan) so that the time vector is approximately monotonously increasing.

    This will invalidate package time to sample relation.
    """
    fs = ds.estimated_frequency
    s = splitby_time(ds, eps_gap)

    news = []

    if len(s) > 1:
        for i in range(len(s) - 1):
            s0 = s[i]
            s1 = s[i + 1]

            # print(s0)
            # print(s1)

            t0 = s0.time.values[-1]
            t1 = s1.time.values[0]
            assert t1 > t0, "this is designed to fill gaps. use retime to handle overlapping samples (i.e. lower than expected sample rate)."

            N = int((t1 - t0).astype('timedelta64[ms]').astype(float) / fs)
            time = np.arange(0, N) / fs
            time = pd.to_timedelta(time, 'ms') + t0
            v = np.full((N, ), fill_value)

            assert N > 0

            fds = xr.Dataset(coords={'time': time, 'package': []})
            for var in s0.data_vars:
                if 'time' in s0[var].dims:
                    fds[var] = xr.DataArray(name=var,
                                            data=v,
                                            dims=('time'),
                                            attrs=s0[var].attrs)
            news.append(s0)
            news.append(fds)

        news.append(s[-1])
        ds = concat(news)

        # fix time (based on first sample (assuming retime already done)
        time = np.arange(0, ds.dims['time']) * 1000. / fs
        time = pd.to_timedelta(time, 'ms') + ds.time.values[0]
        ds = ds.assign_coords(time=('time', time)).reindex({'time': time})

        return ds

    else:
        # no gaps
        return ds


def reproject_pca(ds: xr.Dataset, low=None, high=None):
    """
    Re-project x and y vectors onto direction of maximum variance.
    """
    ds = ds.copy()

    if 'u_x' in ds:
        # re-project displacement
        xx, yy, v0, v1, u0, u1 = signal.reproject_pca(ds.u_x, ds.u_y,
                                                      ds.estimated_frequency,
                                                      low, high)
        ds['u_x'][:] = xx
        ds['u_y'][:] = yy

        ds['u_x']['variance'] = v0
        ds['u_y']['variance'] = v1

    if 'w_x' in ds:
        # re-project acceleration
        xx, yy, v0, v1, u0, u1 = signal.reproject_pca(ds.w_x, ds.w_y,
                                                      ds.estimated_frequency,
                                                      low, high)
        ds['w_x'][:] = xx
        ds['w_y'][:] = yy

        ds['w_x']['variance'] = v0
        ds['w_y']['variance'] = v1

    return ds
