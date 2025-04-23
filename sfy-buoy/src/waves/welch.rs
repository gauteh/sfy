use heapless::Vec;
use static_assertions as sa;

pub const NFFT: usize = 4096;
pub const NSEG: usize = 4096;
pub const NOVERLAP: usize = NSEG / 2;
sa::const_assert!(NOVERLAP < NSEG);

// Cut-off frequencies for spectrum.
pub const f0: f32 = 0.01; // Hz
pub const f1: f32 = 5.0; // Hz

pub const WELCH_PACKET_SZ: usize = 124;

/// Maximum length of base64 string
pub const WELCH_OUTN: usize = { 6 * WELCH_PACKET_SZ * 2 } * 4 / 3 + 4;

pub struct Welch {
    buf: Vec<f32, NSEG>,
    spec: Vec<f32, NFFT>,
    nseg: u16,
}

impl Welch {
    pub fn new() -> Welch {
        Welch {
            buf: Vec::new(),
            spec: Vec::new(),
            nseg: 0,
        }
    }

    /// Returns duration (in seconds) given sample rate.
    pub fn length(&self, fs: f32) -> f32 {
        if self.nseg == 0 {
            return 0.0;
        } else {
            let N = self.nseg - 1;
            let N = NSEG as f32 + (NSEG - NOVERLAP) as f32 * N as f32;

            return N / fs;
        }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.spec.clear();
    }

    /// Add new sample to buf: returns true if segment is full, computed and cleared.
    pub fn sample(&mut self, z: f32) -> bool {
        unsafe { self.buf.push_unchecked(z) };

        if self.buf.is_full() {
            self.compute_seg();
            self.sample(z);

            return true;
        } else {
            return false;
        }
    }

    /// Compute FFT of segment and merge with spectrum. Returns a spectrum if complete.
    pub fn compute_seg(&mut self) {
        // Compute FFT from buf
        use microfft::real::rfft_4096;
        let mut v = self.buf.clone().into_array().unwrap();

        self.buf.clear();

        // Overlap
        self.buf.extend_from_slice(&v[..NOVERLAP]).unwrap();

        // Window?

        // FFT
        let f = rfft_4096(&mut v);

        // Add to spec
        // XXX:
        self.nseg += 1;
    }

    /// Compute Welch-spectrum.
    pub fn finalize_spectrum(&mut self) -> Vec<f32, NFFT> {
        let spec = self.spec.clone();
        self.reset();

        if self.nseg == 0 {
            return spec;
        } else {
            // XXX: divide by self.nseg (if > 0)
            unimplemented!();

            spec
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::*;

    #[test]
    fn test_length() {
        let w = Welch::new();
        assert_eq!(w.length(26.), 0.0);

        let mut w = Welch::new();
        for _ in 0..4096 {
            w.sample(0.0);
        }
        assert_abs_diff_eq!(w.length(26.), 157.53847);

        for _ in 0..4096 {
            w.sample(0.0);
        }
        assert_abs_diff_eq!(w.length(26.), 2.0 * 157.53847);

        for _ in 0..10 {
            for _ in 0..4096 {
                w.sample(0.0);
            }
        }
        assert_abs_diff_eq!(w.length(26.), 1890.46153);
    }
}
