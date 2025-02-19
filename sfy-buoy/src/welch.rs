use heapless::Vec;

pub const NFFT: usize = 4096;
pub const NSEG: usize = 4096;
pub const NOVERLAP: usize = NSEG / 2;

// Cut-off frequencies for spectrum.
pub const f0: f32 = 0.01;   // Hz
pub const f1: f32 = 5.0;      // Hz

pub struct Welch {
    buf: Vec<f64, NSEG>,
    spec: Vec<f64, NFFT>,
}
