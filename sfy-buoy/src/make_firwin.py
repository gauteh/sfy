import scipy as sc, scipy.signal

NTAP = 128      # Length of filter
CUTOFF = 25.    # Cut-off frequency for output
FREQ = 833.     # Input frequency

fir = sc.signal.firwin(NTAP, cutoff=CUTOFF, pass_zero='lowpass', fs = FREQ)

with open('firwin.coeff', 'w') as fd:
    fd.write('[\n')
    for v in fir:
        fd.write('    %.65f,\n' % v)
    fd.write(']')

