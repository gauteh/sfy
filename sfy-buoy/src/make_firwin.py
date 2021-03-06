import scipy as sc, scipy.signal

# 50 Hz
NTAP = 128      # Length of filter
CUTOFF = 25.    # Cut-off frequency for output
FREQ = 208.     # Input frequency

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.25_208_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 20 Hz
NTAP = 128      # Length of filter
CUTOFF = 10.    # Cut-off frequency for output
FREQ = 208.     # Input frequency

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.10_208_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')
