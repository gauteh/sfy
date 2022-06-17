import numpy as np
import scipy as sc, scipy.signal, scipy.integrate

def bandpass(s, dt, low, high):
    fs = 1. / dt
    sos = sc.signal.butter(10, [low, high], 'bandpass', fs=fs, output='sos')
    s = sc.signal.sosfilt(sos, s)
    return s

def integrate(s, dt, detrend=True, filter=True, order=1):
    if order > 1:
        s = integrate(s, dt, detrend, filter, order - 1)

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
        sos = sc.signal.butter(10, [.08, 25.], 'bandpass', fs=fs, output='sos')
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
