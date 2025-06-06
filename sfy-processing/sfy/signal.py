import logging
import numpy as np
import scipy as sc, scipy.signal, scipy.integrate
import xarray as xr

logger = logging.getLogger(__name__)


def adjust_fir_filter(x: xr.Dataset, inplace=True):
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

    x.attrs['fir_adjusted'] = delay.item().total_seconds() * 1000
    x.attrs['fir_adjusted:unit'] = 'ms'

    return x


DEFAULT_BANDPASS_FREQS_52Hz = [0.05, 25.]
DEFAULT_BANDPASS_FREQS_20Hz = [0.05, 10.]


def bandpass(s, dt, low=None, high=None):
    fs = 1. / dt

    if low is None:
        if fs < 50:
            low = DEFAULT_BANDPASS_FREQS_20Hz[0]
        else:
            low = DEFAULT_BANDPASS_FREQS_52Hz[0]

    if high is None:
        if fs < 50:
            high = DEFAULT_BANDPASS_FREQS_20Hz[1]
        else:
            high = DEFAULT_BANDPASS_FREQS_52Hz[1]

    # Filtering once each direction, doubling the order with 0 phase shift.
    sos = sc.signal.butter(10, [low, high], 'bandpass', fs=fs, output='sos')
    s = sc.signal.sosfiltfilt(sos, s)
    return s


def integrate(s,
              dt,
              detrend=True,
              filter=True,
              order=1,
              freqs=None,
              method='dft'):
    """
    Integrate a signal, first removing mean and detrending.

    Args:

        s: signal

        dt: sample rate, 1 / Fs.

        detrend: remove mean and detrend before integrating.

        filter: filter before integrating.

        order: number of times to perform integration recursively.

        freqs: list with upper and lower bound for filter.

        method: numerical integration method: 'trapz', 'dft' (default).

    Returns:

        s: integrated signal.
    """
    assert order > 0

    fs = 1. / dt

    if order > 1:
        s = integrate(s, dt, detrend, filter, order - 1, freqs)

    if freqs is None:
        if fs < 50:
            freqs = DEFAULT_BANDPASS_FREQS_20Hz
        else:
            freqs = DEFAULT_BANDPASS_FREQS_52Hz

    ## Detrend
    if detrend:
        s = sc.signal.detrend(s)
        s = s - np.mean(s)

    ## Filter
    # # Use elliptic filter (https://github.com/jthomson-apluw/SWIFT-codes/blob/master/Waves/rawdisplacements.m)
    # (b, a) = sc.signal.ellip(3, .5, 20, 0.1, 'highpass', fs = fs)
    # (b, a) = sc.signal.butter(8, 0.05, 'highpass', fs=fs)
    # s = sc.signal.filtfilt(b, a, s)

    if filter:
        s = bandpass(s, dt, *freqs)

    ## Integrate
    if method == 'trapz':
        s = sc.integrate.cumulative_trapezoid(s, dx=dt)
    elif method == 'dft':
        s = dft_integrate(s, fs)
    else:
        raise ValueError("Unknown integration method")

    return s


def dft_integrate(x, fs):
    """
    Integrate in the Fourier domain. See Brandt & Brincker (2014) for a comparsion with the trapezoidal rule.
    """

    L = len(x)
    N = 2 * L  # x should be padded to avoid cyclic aliasing, achieved through taking the DFT at 2*L.

    X = np.fft.rfft(x, N)

    ## Integrator operator
    f = np.fft.rfftfreq(N, d=1. / fs)
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


def spectral_moment(f, H, order=0):
    """
    Calculate the spectral moment `m_o` of `order`.

    Args:

        f: frequencies

        H: Elevation energies (array of real floats)

        order: Order of spectral moment

    Returns:

        (float): spectral moment
    """

    # Cut spectra to avoid infinite values from low-frequencies.
    f0 = 1. / (20 * 60)  # T = 20 minutes
    ff = f >= f0

    f = f[ff]
    H = H[..., ff]

    assert len(f) > 10

    M = np.power(f, order) * H
    return np.trapz(M, f)


def welch(freq, e, nperseg=4096, order=2):
    """
    Wrapper around `scipy.signal.welch` (with sane default parameters) that integrates the spectrum twice (by default).

    Args:
        order: Integration order, default 2 (assuming input is acceleration)
    """
    f, P = scipy.signal.welch(e,
                              freq,
                              nperseg=nperseg,
                              nfft=nperseg,
                              detrend='linear')

    if order > 0:
        P = welchint(f, P, order)

    assert len(P) == nperseg // 2 + 1, f'{len(P)}, {nperseg}'

    return f, P


def hm0(f, H):
    """
    Calculate Hm0. See `ref:hs` for definition.

    Args:
        f: Frequencies.

        H: Elevation energies

    Returns:

        Significant wave height estimated through the first order spectral moment.

    Note:

        > According to [0] studies show that Hm0 overestimates Hs with about 5%.

    [0](https://support.nortekgroup.com/hc/en-us/articles/360029507012-What-is-the-difference-between-Hm0-and-Hs-)
    """

    m0 = spectral_moment(f, H, 0)
    return 4 * np.sqrt(m0)


def spec_stats(f, H):
    """
    Calculate Hm0, m0, m1, Tc, Tz, etc. See `ref:hs` for definition.

    Based on code in `decoder.py` from OMB. And:
        * Holthuijsen, Leo H. Waves in Oceanic and Coastal Waters. Cambridge University Press, 2010.
        * Bidlot, Jean-Raymond. “Ocean Wave Model Output Parameters,” 2016.
    Args:
        f: Frequencies.

        H: Elevation energies

    Returns:

        Spectral moments and derived parameters.
    """

    m_1 = spectral_moment(f, H, -1)
    m0 = spectral_moment(f, H, 0)
    m1 = spectral_moment(f, H, 1)
    m2 = spectral_moment(f, H, 2)
    m4 = spectral_moment(f, H, 4)

    hm0 = 4 * np.sqrt(m0)
    Tm01 = (m0 / m1)  # mean zero-crossing period (Holthuijsen)
    Tm02 = np.sqrt(m0 / m2)  # mean zero-crossing period (Holthuijsen)
    Tm_10 = m_1 / m0  # mean wave period (inverse frequency moment)
    Tp = 1. / f[np.argmax(H)]  # peak period

    return m_1, m0, m1, m2, m4, hm0, Tm01, Tm02, Tm_10, Tp


def hs(e):
    """
    Estimate Hs through the standard deviation of the signal.

    > The significant wave height is defined and calculated as the mean of the top 1/3 waves in a given record.

    Args:

        e: elevation

    Returns:

        (float) hs
    """
    return 4 * np.nanstd(e)


def welchint(f, P, order=2):
    """
    Integrate a Welch _acceleration_ spectrum to an _elevation_ spectrum.

    Args:

        f: frequencies for which P is given (Hz)

        P: Frequency amplitude of acceleration spectra.

        order: Integration order (default 2, acceleration to elevation).
    """
    if order == 0:
        return P

    order = 2 * order
    D = np.power((2 * np.pi * f), order)
    I = D > 0
    P[I] = P[I] / D[I]

    return P


def imu_cutoff_rabault2022(f, E, f0=0.05):
    """
    Find lower cutoff frequency of IMU based measurements based on Figure 7 in Rabault (2022) (https://www.mdpi.com/2076-3263/12/3/110).

    Based on: https://github.com/jerabaul29/OpenMetBuoy-v2021a/blob/1ae44ad9b9ee06b35e36f6f281cb9cf1dd029373/legacy_firmware/decoder/decoder.py#L200

    Args:

        f: frequencies

        E: Elevation spectrum

        f0: Discard energy below this frequency (default: 0.05 Hz)

    Returns:

        i, f, P: index in f and f of low frequency cutoff, and cut spectrum.
    """

    print("freq=", f, f.shape)

    OMB_df = 0.048828125 - 0.0439453125  #  df for OpenMetBuoy

    df = f[1] - f[0]
    assert np.max(
        np.abs(df - np.diff(f))
    ) < 1e-12, f"df not constant: {df} vs {np.diff(f)}, max diff: {np.max(np.abs(np.diff(f)-df))}"

    # Below f0 (0.05 Hz) the signal becomes very noisy, and quadrubly so because of the integration.
    if0 = np.argmax(f >= f0)
    # assert if0 > 0 and f0 <= f[0]

    N = len(f)

    fp, f = np.array_split(f, [if0])
    Ep, E = np.array_split(E, [if0])

    NE = -E / np.max(E)  # normalized spectrum

    distance = int(3 * (df / OMB_df))
    peaks, _ = scipy.signal.find_peaks(NE, distance=distance, prominence=0.05)
    peak = peaks[0] if len(peaks) > 0 else 0  # the first peak

    # If there is no clear minimum, keep the entire spectrum.
    if E[peak] > 0.1:
        peak = 0

    # Do not flag out valid parts of the spectrum when the spectrum is "really clean"
    # in cases where the spectrum is "really clean", can happen that the first minimum is a local minimum after the first valid peak
    # detect these cases and set the full spectrum as valid then
    if (E[peak] > ((E[0] + E[1]) / 2.0)):
        peak = 0

    EE = np.zeros((len(Ep) + len(E), ))
    EE[(if0 + peak):] = E[peak:]

    return (if0 + peak), f[peak], EE


def reproject_pca(x, y, Fs=None, low=None, high=None):
    """
    Re-project x-y vectors in direction of maximum variance using Principal
    Component Analysis (sfy-paper). The primary direction will be aligned along
    the 'x' direction.

    For this to work well the direction and gyro-drift should be relative
    stable for the segment of the signal. 10 seconds in the wave-flume works
    ok, for more chaotic situations this might not work so well.

    It also works better to re-project the high-freq movement.

    * https://math.stackexchange.com/questions/2398662/how-to-find-the-axis-of-highest-variance-of-a-set-of-2d-points
    * https://stackoverflow.com/questions/73652615/is-there-a-rolling-implementation-of-pca-in-python/73652616#73652616

    Args:

        x, y: vector components to re-project

        low, high: calculate variance on filtered signal between these
        frequencies. If one is set and one is None, the default value is used.


    Returns:

        xx, yy, varx, vary, u0, u1

        x and y re-projected so that x is along the direction of major
        variance. y is orthogonal to x.

        varx and vary is the explained variance of the two components. the
        ratio between the first and second gives an idea about
        how little the gyro is drifting, or how multi-directional the wave is.

        u0, u1 is the new basis.
    """
    from sklearn.decomposition import PCA
    pca = PCA(n_components=2, whiten=True, copy=True)

    if low is not None or high is not None:
        assert Fs is not None, "Fs must be set when filtering"
        xb = bandpass(x, 1. / Fs, low, high)
        yb = bandpass(y, 1. / Fs, low, high)

        U = np.vstack((xb, yb)).T
    else:
        U = np.vstack((x, y)).T

    pca.fit(U)

    C = pca.components_

    # Find direction:
    u0 = C[0]  # greatest variation
    u1 = C[1]  # second, orthogonal to first

    # normalize
    u0 = u0 / np.linalg.norm(u0)
    u1 = u1 / np.linalg.norm(u1)

    assert np.isclose(u0.dot(u1), 0., atol=1e-6), "PCAs should be orthogonal"

    U = np.vstack((x, y)).T  # re-assign to unfiltered vectors.
    UU = np.vstack((U.dot(u0), U.dot(u1))).T
    xx, yy = UU[:, 0], UU[:, 1]

    return xx, yy, pca.explained_variance_[0], pca.explained_variance_[
        1], u0, u1
