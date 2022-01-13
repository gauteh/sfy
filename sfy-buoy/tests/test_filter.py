import numpy as np
import scipy as sc, scipy.signal
import matplotlib.pyplot as plt

def test_5hz_fir():
    fs = 833.
    dt = 1. / fs
    co = 25.
    ntap = 128

    fir = sc.signal.firwin(ntap, cutoff=co, pass_zero='lowpass', fs = fs)

    t = np.arange(0, dt * 4096, dt)
    s = 2 * np.sin(40. * t * np.pi * 2)

    zf = np.convolve(s, fir, mode = 'same')

    assert len(fir) == 128

    # running
    sf = []
    for i in range(128//2):
        sf.append(0)

    for i in range(128, len(s)):
        win = s[i-128:i]
        o = np.sum(win * fir)
        sf.append(o)

    # plt.plot(s, label = 'orig')
    # plt.plot(zf, label = 'filterd (conv)')
    # plt.plot(sf, label = 'filterd (running)')
    # plt.legend()
    # plt.show()


