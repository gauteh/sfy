import numpy as np
from scipy import signal
import matplotlib.pyplot as plt

# make a few examples we can test against in sfy-buoy

fs = 26.
nseg = 4096
nfft = nseg
noverlap = nseg / 2


def write_test(id, time, s, f, pxx):
    np.savez(f'welch_test_{id}.npz', time=time, s=s, f=f, pxx=pxx)


amp = 2.
freq = 1.3
noise_power = 0.001

time = np.arange(0, fs * 60 * 20) / fs

rng = np.random.default_rng()
s = amp * np.sin(2 * np.pi * freq * time)
s += rng.normal(scale=np.sqrt(noise_power), size=time.shape)

f, Pxx_den = signal.welch(s, fs, nperseg=nseg, nfft=nfft, noverlap=noverlap)

write_test(1, time, s, f, Pxx_den)

plt.semilogy(f, Pxx_den)
plt.ylim([0.5e-5, 100])
plt.xlabel('frequency [Hz]')
plt.ylabel('PSD [V**2/Hz]')
plt.show()
