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

