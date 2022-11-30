import scipy as sc

# 50 Hz
NTAP = 129      # Length of filter
CUTOFF = 20.    # Cut-off frequency for output
FREQ = 208.     # Input frequency

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.25_208_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 20 Hz
NTAP = 129      # Length of filter
CUTOFF = 8.    # Cut-off frequency for output
FREQ = 208.     # Input frequency

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.10_208_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')
