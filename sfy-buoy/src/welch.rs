use heapless::Vec;

pub const NFFT: usize = 4096;
pub const NSEG: usize = 4096;
pub const NOVERLAP: usize = NSEG / 2;

// Cut-off frequencies for spectrum.
pub const f0: f32 = 0.01;   // Hz
pub const f1: f32 = 5.0;      // Hz

pub const WELCH_PACKET_SZ: usize = 124;

/// Maximum length of base64 string
pub const WELCH_OUTN: usize = { 6 * WELCH_PACKET_SZ * 2 } * 4 / 3 + 4;

pub struct Welch {
    buf: Vec<f64, NSEG>,
    spec: Vec<f64, NFFT>,
}
