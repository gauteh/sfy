import logging
import numpy as np
import scipy as sc, scipy.signal, scipy.integrate
import xarray as xr

logger = logging.getLogger(__name__)

def adjust_fir_filter(x: xr.Dataset, inplace = True):
    """
    Adjust for FIR filter.

    Args:

        x: xarray Dataset or DataArray
    """
    if 'fir_adjusted' in x.attrs:
        logger.error('Dataset already adjusted for FIR filter delay')
        return x

    if not inplace:
        x = x.copy(deep=True)

    NTAP = 128
    Fs = 208.
    delay = ((NTAP / 2) / Fs) * 1000
    delay = np.timedelta64(int(delay), 'ms')
    x['time'].values[:] = x['time'].values[:] + delay

    x.attrs['fir_adjusted'] = delay

    return x


def bandpass(s, dt, low=None, high=None):
    fs = 1. / dt

    if low is None:
        low = 0.08

    if high is None:
        high = 25.

    # Filtering once each direction, doubling the order with 0 phase shift.
    sos = sc.signal.butter(10, [low, high], 'bandpass', fs=fs, output='sos')
    s = sc.signal.sosfiltfilt(sos, s)
    return s

def integrate(s, dt, detrend=True, filter=True, order=1, freqs=None, method='trapz'):
    """
    Integrate a signal, first removing mean and detrending.

    Args:

        s: signal

        dt: sample rate, 1 / Fs.

        detrend: remove mean and detrend before integrating.

        filter: filter before integrating.

        order: number of times to perform integration recursively.

        freqs: list with upper and lower bound for filter.

        method: numerical integration method: 'trapz', 'dft'.

    Returns:

        s: integrated signal.
    """
    if order > 1:
        s = integrate(s, dt, detrend, filter, order - 1, freqs)

    if freqs is None:
        freqs = [.05, 25.]

    fs = 1. / dt

    ## Detrend
    if detrend:
        s = s - np.mean(s)
        s = sc.signal.detrend(s)

    ## Filter
    # # Use elliptic filter (https://github.com/jthomson-apluw/SWIFT-codes/blob/master/Waves/rawdisplacements.m)
    # (b, a) = sc.signal.ellip(3, .5, 20, 0.1, 'highpass', fs = fs)
    # (b, a) = sc.signal.butter(8, 0.05, 'highpass', fs=fs)
    # s = sc.signal.filtfilt(b, a, s)

    if filter:
        s = bandpass(s, dt, *freqs)

    ## Integrate
    if method == 'trapz':
        s = sc.integrate.cumtrapz(s, dx=dt)
    elif method == 'dft':
        s = dft_integrate(s, fs)
    else:
        raise ValueError("Unknown integration method")

    return s


def velocity(a: 'Axl'):
    """
    Calculate velocity from axelerometer package. Resulting vectors will be one length shorter than the original.
    """
    x = integrate(a.x, a.dt)
    y = integrate(a.y, a.dt)
    z = integrate(a.z, a.dt)

    return x, y, z


def displacement(a: 'Axl'):
    """
    Calculate diplacement from axelerometer package. Resulting vectors will be two length shorter than the original.
    """
    x = integrate(a.x, a.dt, order=2)
    y = integrate(a.y, a.dt, order=2)
    z = integrate(a.z, a.dt, order=2)

    return x, y, z

def dft_integrate(x, fs):
    """
    Integrate in the Fourier domain. See Brandt & Brincker (2014) for a comparsion with the trapezoidal rule.
    """

    L = len(x)
    N = 2 * L # x should be padded to avoid cyclic aliasing, achieved through taking the DFT at 2*L.

    X = np.fft.rfft(x, N)

    ## Integrator operator
    f = np.fft.rfftfreq(N, d = 1. / fs)
    w = 2. * np.pi * f
    H = np.empty(shape=w.shape, dtype=complex)
    H[1:] = 1. / (1j * w[1:])
    H[0] = 0.

    ## Integrate
    Y = X * H

    y = np.fft.irfft(Y)
    y = y[:L]

    return y

def detrend_tp_2021(y, k=0.9995):
    """
    Detrend signal using algorithm from Tucker and Pitt (2001), `k` from Kohout et. al. (2015).

    Apparently this should be equivalen to a highpass RC-filter.

    .. math::

        y^{*}_{n} = y_n - (1 - k) * s_n
        s_n = y_n + k * s_{n-1}

        y_n is the raw signal
        y^{*}_{n} is the detrended signal
    """
    raise NotImplemented()
