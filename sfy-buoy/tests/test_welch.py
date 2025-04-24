import numpy as np
from scipy import signal
import matplotlib.pyplot as plt

def test_welch_1():
    pyd = np.load('tests/data/welch/welch_test_1.npz')
    rsd = {}
    exec(open('tests/data/welch/welch_test_1_rust_pxx').read(), rsd)

    plt.semilogy(pyd['f'], pyd['pxx'], label='py')
    plt.semilogy(pyd['f'][:-1], rsd['pxx'], label='rs')
    plt.ylim([0.5e-5, 100])
    plt.xlabel('frequency [Hz]')
    plt.ylabel('PSD [V**2/Hz]')
    plt.legend()
    plt.show()
