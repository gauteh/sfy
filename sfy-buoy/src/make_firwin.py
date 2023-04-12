import scipy as sc

FREQ = 208.     # Input frequency

# 50 Hz
NTAP = 129      # Length of filter
CUTOFF = 20.    # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open(f'firwin.25_{FREQ:.0f}_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

# 20 Hz
NTAP = 129      # Length of filter
CUTOFF = 8.     # Cut-off frequency for output

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open(f'firwin.10_{FREQ:.0f}_coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')
