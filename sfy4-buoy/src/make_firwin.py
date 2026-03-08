import scipy as sc

FREQ = 208.     # Input frequency

# 52 Hz
NTAP = 129      # Length of filter
CUTOFF = 26.    # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.26_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 26 Hz
NTAP = 129      # Length of filter
CUTOFF = 13.    # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.13_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 13 Hz
NTAP = 129      # Length of filter
CUTOFF = 6.5    # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.6.5_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')
