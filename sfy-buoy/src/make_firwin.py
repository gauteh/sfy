import scipy as sc

FREQ = 417.     # Input frequency

# 50 Hz
NTAP = 129      # Length of filter
CUTOFF = 26.    # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.25_52_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 20 Hz
NTAP = 129      # Length of filter
CUTOFF = 13.    # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.10_52_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')
