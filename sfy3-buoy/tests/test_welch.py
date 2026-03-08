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

def test_sinc_interp():
    pyd = np.load('tests/data/welch/welch_test_1.npz')
    f = pyd['f']
    pxx = pyd['pxx']

    def interp (xp, xt, x):
        """
        Interpolate the signal to the new points using a sinc kernel

        input:
        xt    time points x is defined on
        x     input signal column vector or matrix, with a signal in each row
        xp    points to evaluate the new signal on

        output:
        y     the interpolated signal at points xp
        """

        mn = x.shape
        if len(mn) == 2:
            m = mn[0]
        elif len(mn) == 1:
            m = 1
        else:
            raise ValueError ("x is greater than 2D")

        nn = len(xp)

        y = np.zeros((m, nn))

        for (pi, p) in enumerate (xp):
            si = np.tile(np.sinc (xt - p), (m, 1))
            y[:, pi] = np.sum(si * x)

        return y.squeeze ()

    fp = f[::10]
    spxx = interp(fp, f, pxx)
    plt.semilogy(f, pxx, label='py')
    plt.semilogy(fp, spxx, label='interp')
    plt.ylim([0.5e-5, 100])
    plt.xlabel('frequency [Hz]')
    plt.ylabel('PSD [V**2/Hz]')
    plt.legend()
    plt.show()


def test_downsample_psd():
    pyd = np.load('tests/data/welch/welch_test_1.npz')
    f = pyd['f']
    pxx = pyd['pxx']

    df = np.diff(f)[0]
    ds = 10
    fp = f[::ds]
    spxx = []
    for i in range(int(len(pxx)/ds)):
        s = pxx[i*ds:(i+1)*ds]
        s = np.sum(s)/(ds)
        spxx.append(s)

    plt.semilogy(f, pxx, label='py')
    plt.semilogy(fp[:-1], spxx, label='interp')
    plt.ylim([0.5e-5, 100])
    plt.xlabel('frequency [Hz]')
    plt.ylabel('PSD [V**2/Hz]')
    plt.legend()
    plt.show()
