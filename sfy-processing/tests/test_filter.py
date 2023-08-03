from sfy import axl
import numpy as np
import scipy as sc, scipy.signal
import matplotlib.pyplot as plt

def test_fir():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()

    a = axl.Axl.parse(d)

    fir = sc.signal.firwin(128, cutoff=25., pass_zero='lowpass', fs = a.freq)
    print(fir.shape)

    zf = np.convolve(a.z, fir, mode = 'same')
    # plt.plot(a.z, label = 'orig')
    # plt.plot(zf, label = 'filterd')
    # plt.legend()
    # plt.show()

def test_filter_coeffs():
    f50 = eval(open('../sfy-buoy/src/firwin.26_coeff').read())
    f50 = np.array(f50)

    NTAP = 129      # Length of filter
    CUTOFF = 26.    # Cut-off frequency for output
    FREQ = 208.     # Input frequency
    fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

    np.testing.assert_allclose(fir, f50, rtol=1e-10)

    f20 = eval(open('../sfy-buoy/src/firwin.13_coeff').read())
    f20 = np.array(f20)

    NTAP = 129      # Length of filter
    CUTOFF = 13.    # Cut-off frequency for output
    FREQ = 208.     # Input frequency
    fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

    np.testing.assert_allclose(fir, f20, rtol=1e-10)

def test_fir_within_sin():
    NTAP = 129      # Length of filter
    CUTOFF = 20.    # Cut-off frequency for output
    FREQ = 208.     # Input frequency
    fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

    dt = 1. / FREQ
    t = np.arange(0, 4096) * dt
    s = 2 * np.sin(2 * t * 2 * np.pi)

    sf = np.convolve(s, fir, mode='valid')
    # sf = sf[NTAP//2:]

    # import matplotlib.pyplot as plt
    # plt.figure()
    # plt.plot(np.arange(0,len(s)), s, '-x')
    # plt.plot(np.arange(0,len(sf))+64, sf, '-x')
    # plt.show()

    # sf = sf[:]
    s = s[64:-64]

    np.testing.assert_almost_equal(sf, s, 2)

def test_fir_outside_sin():
    NTAP = 129      # Length of filter
    CUTOFF = 20.    # Cut-off frequency for output
    FREQ = 208.     # Input frequency
    fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

    dt = 1. / FREQ
    t = np.arange(0, 4096) * dt

    s = 2 * np.sin(2 * t * 2 * np.pi)
    s2 = 2 * np.sin(50 * t * 2 * np.pi)
    s3 = 2 * np.sin(25 * t * 2 * np.pi)

    sf = np.convolve(s+s2+s3, fir, mode='valid')
    # sf = sf[NTAP//2:]

    # import matplotlib.pyplot as plt
    # plt.figure()
    # plt.plot(np.arange(0,len(s)), s, '-x')
    # plt.plot(np.arange(0,len(sf))+64, sf, '-x')
    # plt.show()

    # sf = sf[:]
    s = s[64:-64]

    np.testing.assert_almost_equal(sf, s, 2)
