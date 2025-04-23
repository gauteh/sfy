use core::f32::consts::PI;

use heapless::Vec;
use num_complex::ComplexFloat;
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
    /// Frequency
    fs: f32,

    /// Rolling segment. When full, added to spec.
    buf: Vec<f32, NSEG>,

    /// Real side of spectrum.
    spec: Vec<f32, { NFFT / 2 }>,

    /// Total number of segments (buf's) that have gone into the spectrum.
    nseg: u16,
}

impl Welch {
    pub fn new(fs: f32) -> Welch {
        let mut w = Welch {
            fs,
            buf: Vec::new(),
            spec: Vec::new(),
            nseg: 0,
        };

        w.reset();

        w
    }

    /// Returns duration (in seconds) given sample rate.
    pub fn length(&self) -> f32 {
        if self.nseg == 0 {
            return 0.0;
        } else {
            let N = self.nseg - 1;
            let N = NSEG as f32 + (NSEG - NOVERLAP) as f32 * N as f32;

            return N / self.fs;
        }
    }

    /// Δf between frequency bins.
    pub fn frequency_resolution(&self) -> f32 {
        self.fs / NFFT as f32
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.spec.clear();
        self.spec.resize(NFFT / 2, 0.0).unwrap();
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
    ///
    /// Computes the energy spectrum [m^2/Hz] for the current segment, and adds it to the
    /// total spectrum (which needs to be divided by the number of segments to find the
    /// average).
    pub fn compute_seg(&mut self) {
        // Compute FFT from buf
        use microfft::real::rfft_4096;
        let mut v = self.buf.clone().into_array().unwrap();

        self.buf.clear();

        // Copy end to next segment, so that segments overlap.
        self.buf
            .extend_from_slice(&v[(NSEG - NOVERLAP + 1)..])
            .unwrap();

        // Window & detrend: Hanning window
        for (i, vv) in v.iter_mut().enumerate() {
            *vv = HANNING_AMPLITUDE_CORRECTION
                * hanning(i, NSEG)
                * (*vv - super::buf::SENSORS_GRAVITY_STANDARD as f32);
        }

        // FFT
        let f = rfft_4096(&mut v);
        debug_assert_eq!(f.len(), self.spec.len());
        debug_assert_eq!(f.len(), NFFT / 2);

        // Add energy to spectrum
        let fsr = self.frequency_resolution();

        for (v, s) in f.iter().zip(self.spec.iter_mut()) {
            let e = (v * v.conj()).re(); // energy
            *s += e / (NFFT * NFFT) as f32 / fsr;
        }

        self.nseg += 1;
    }

    /// Compute Welch-spectrum (WARNING: does not reset).
    pub fn compute_spectrum(&mut self) -> Vec<f32, { NFFT / 2 }> {
        let mut spec = self.spec.clone();

        if self.nseg == 0 {
            return spec;
        } else {
            for v in &mut spec {
                *v = *v / self.nseg as f32;
            }

            spec
        }
    }

    /// Compute Welch-spectrum and reset spectrum.
    pub fn take_spectrum(&mut self) -> Vec<f32, { NFFT / 2 }> {
        let spec = self.compute_spectrum();
        self.reset();

        spec
    }
}

/// Hanning-window.
pub fn hanning(i: usize, N: usize) -> f32 {
    assert!(i < N);
    0.5 - 0.5 * f32::cos((2.0 * PI * i as f32) / (N - 1) as f32)
}

pub const HANNING_ENERGY_CORRECTION: f32 = 1.63; // for large N
pub const HANNING_AMPLITUDE_CORRECTION: f32 = 2.0; // for large N

#[cfg(test)]
mod tests {
    use super::*;
    use approx::*;

    #[test]
    fn test_length() {
        let w = Welch::new(26.);
        assert_eq!(w.length(), 0.0);

        let mut w = Welch::new(26.);
        for _ in 0..4096 {
            w.sample(0.0);
        }
        assert_abs_diff_eq!(w.length(), 157.53847);

        for _ in 0..4096 {
            w.sample(0.0);
        }
        assert_abs_diff_eq!(w.length(), 2.0 * 157.53847);

        for _ in 0..10 {
            for _ in 0..4096 {
                w.sample(0.0);
            }
        }
        assert_abs_diff_eq!(w.length(), 1890.46153);
    }

    #[test]
    fn test_overlap() {
        let mut w = Welch::new(26.);

        let N = 26 * 20 * 60; // 20 minutes
        let mut n = 0;

        for i in 0..N {
            let s = w.sample(0.0);
            n += 1;

            if s {
                assert_eq!(w.buf.len(), NOVERLAP);
            }

            println!("{i} ({n}) => {s}");

            // first segment, no overlap
            if i < (NSEG - 1) {
                assert!(!s);
            } else {
                if n == NSEG {
                    assert!(s); // first segment
                    n = 0;
                } else {
                    if n % (NSEG - NOVERLAP) == 0 {
                        assert!(s); // new segment
                        n = 0;
                    } else {
                        assert!(!s);
                    }
                }
            }
        }

        let t = 20.0 * 60.0 - (w.buf.len() - NOVERLAP) as f32 / 26.;
        assert_abs_diff_eq!(w.length(), t);
    }
}
