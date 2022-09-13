import numpy as np
import scipy as sc, scipy.signal, scipy.integrate

def bandpass(s, dt, low, high):
    fs = 1. / dt
    sos = sc.signal.butter(10, [low, high], 'bandpass', fs=fs, output='sos')
    s = sc.signal.sosfilt(sos, s)
    return s

def integrate(s, dt, detrend=True, filter=True, order=1, freqs=None):
    """
    Integrate a signal, first removing mean and detrending.
    """
    if order > 1:
        s = integrate(s, dt, detrend, filter, order - 1, freqs)

    if freqs is None:
        freqs = [.08, 25.]

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

    print(f"{freqs=}")

    if filter:
        sos = sc.signal.butter(10, freqs, 'bandpass', fs=fs, output='sos')
        s = sc.signal.sosfilt(sos, s)

    ## Integrate
    s = sc.integrate.cumtrapz(s, dx=dt)

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

def integrate_dft(x, fs):
    """
    Integrate in the Fourier domain. See Brandt & Brincker (2014) for a comparsion with the trapezoidal rule.
    """

    L = len(x)
    N = 2 * L # x should be padded to avoid cyclic aliasing, achieved through taking the DFT at 2*L.

    X = np.fft.rfft(x, N)

    f = np.fft.rfftfreq(N, d = 1. / fs)
    w = 2. * np.pi * f
    H = np.empty(shape=w.shape, dtype=complex)
    H[1:] = 1. / (1j * w[1:])
    H[0] = 0.


    Y = X * H  # integrate

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
    pass
