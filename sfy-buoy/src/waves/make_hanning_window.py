import numpy as np

NSEG = 4096

win = np.hanning(NSEG)
acorr = NSEG / np.sum(win)
ecorr = np.sqrt(NSEG / np.sum(win * win))
sum = np.sum(win)
sum_sqr = np.sum(win * win)

with open(f'hanning_win_{NSEG}.coeff', 'w') as fd:
    fd.write(f'pub const NSEG: usize = {NSEG};\n')
    fd.write(f'pub const ACORR: f32 = {acorr:.65f};\n')
    fd.write(f'pub const ECORR: f32 = {ecorr:.65f};\n')
    fd.write(f'pub const CSUM: f32 = {sum:.65f};\n')
    fd.write(f'pub const CSQRSUM: f32 = {sum_sqr:.65f};\n')
    fd.write('pub const COEFFS: [f32; NSEG] = ')
    fd.write('[\n')
    for v in win:
        fd.write('    %.65f,\n' % v)
    fd.write('];')
