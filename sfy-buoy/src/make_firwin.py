import scipy as sc

FREQ = 208.  # Input frequency
HP_CUTOFF = 1. / 30.  # High-pass cut-off. We do not expect to measure meaningful signals on longer periods than 30 seconds.

# 50 Hz
NTAP = 129  # Length of filter
CUTOFF = 26.  # Cut-off frequency for output

fir = sc.signal.firwin(NTAP,
                       cutoff=[HP_CUTOFF, CUTOFF],
                       pass_zero='bandpass',
                       fs=FREQ)

with open('firwin.26_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 20 Hz
NTAP = 129  # Length of filter
CUTOFF = 13.  # Cut-off frequency for output

fir = sc.signal.firwin(NTAP,
                       cutoff=[HP_CUTOFF, CUTOFF],
                       pass_zero='bandpass',
                       fs=FREQ)

with open('firwin.13_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')
