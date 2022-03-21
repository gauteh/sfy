#! /usr/bin/env python

import sys
import scipy as sc, scipy.signal, scipy.integrate
import numpy as np
import matplotlib.pyplot as plt
from sfy import axl

f = sys.argv[1]
print(f"opening file {f}..")

ax = axl.Axl.from_file(f)
print(ax)

dt = 1. / ax.freq
# w = np.trapz(ax.z, dx = dt)
a = sc.signal.detrend(ax.z[256:])
w = sc.integrate.cumtrapz(a, dx = dt)
z = sc.integrate.cumtrapz(w, dx = dt)

plt.figure()
plt.title(f"{ax.start} / {ax.received_dt} length: {ax.duration}s f={ax.freq}Hz")
plt.plot(ax.mseconds[256:], a, label = 'a_z')
plt.plot(ax.mseconds[256:-1], w, label = 'w_z')
plt.plot(ax.mseconds[256:-2], z, label = 'z')
plt.grid()
plt.legend()
plt.show()
