use heapless::Vec;
use num_complex::ComplexFloat;
use static_assertions as sa;

pub const NFFT: usize = 4096;
pub const NSEG: usize = 4096;
pub const NOVERLAP: usize = NSEG / 2;
sa::const_assert!(NOVERLAP < NSEG);

pub mod hanning {
    use core::f32::consts::PI;
    use static_assertions as sa;

    // TODO: May be too big to include.
    include!("hanning_win_4096.coeff");
    sa::const_assert_eq!(super::NSEG, NSEG);

    /// Hanning-window.
    pub fn hanning(i: usize, N: usize) -> f32 {
        assert!(i < N);
        0.5 - 0.5 * f32::cos((2.0 * PI * i as f32) / (N - 1) as f32)
    }

    pub const HANNING_ENERGY_CORRECTION: f32 = 1.63319253834869915209537793998606503009796142578125; // for large N: NSEG/sum(window)
    pub const HANNING_AMPLITUDE_CORRECTION: f32 = 2.00048840048840048666534130461513996124267578125;
    // for large N: NSEG/sum(window*window)
}

// Cut-off frequencies for spectrum.
pub const f0: f32 = 0.04; // Hz
pub const f1: f32 = 2.0; // Hz

pub const WELCH_PACKET_SZ: usize = 124;

/// Maximum length of base64 string
pub const WELCH_OUTN: usize = { 6 * WELCH_PACKET_SZ * 2 } * 4 / 3 + 4;

/// Rolling Welch spectrum computation (PSD, density mode). Based on scipy.welch implementation.
pub struct Welch {
    /// Frequency
    fs: f32,

    /// Rolling segment. When full, added to spec.
    buf: Vec<f32, NSEG>,
    mean: f32,

    /// Real side of spectrum.
    spec: Vec<f32, { NFFT / 2 }>,
    scaling: f32,

    /// Total number of segments (buf's) that have gone into the spectrum.
    nseg: u16,
}

impl Welch {
    pub fn new(fs: f32) -> Welch {
        let scaling = 1.0 / (fs * hanning::CSQRSUM);
        let scaling = 2.0 * scaling; // onesided / psd

        let mut w = Welch {
            fs,
            buf: Vec::new(),
            mean: 0.0,
            spec: Vec::new(),
            scaling,
            nseg: 0,
        };

        w.reset();

        w
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.spec.clear();
        self.spec.resize(NFFT / 2, 0.0).unwrap();
        self.nseg = 0;
        self.mean = 0.0;
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

    /// Frequency bins
    pub fn rfftfreq(&self) -> [f32; NFFT / 2] {
        sa::const_assert_eq!(NFFT % 2, 0);

        let fsr = self.frequency_resolution();

        let mut f = [0f32; NFFT / 2];
        for (i, ff) in f.iter_mut().enumerate() {
            *ff = i as f32 * fsr;
        }

        f
    }

    /// Add new sample to buf: returns true if segment is full, computed and cleared.
    pub fn sample(&mut self, z: f32) -> bool {
        unsafe { self.buf.push_unchecked(z) };
        self.mean += z / NSEG as f32;

        if self.buf.is_full() {
            self.compute_seg();

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
        self.buf.extend_from_slice(&v[(NSEG - NOVERLAP)..]).unwrap();

        // Window & detrend: Hanning window
        for (i, vv) in v.iter_mut().enumerate() {
            *vv = hanning::COEFFS[i] * (*vv - self.mean);
        }

        // FFT
        let f = rfft_4096(&mut v);
        debug_assert_eq!(f.len(), self.spec.len());
        debug_assert_eq!(f.len(), NFFT / 2);

        // quoting microfft docs:
        // "since the real-valued coefficient at the Nyquist frequency is packed into the
        //  imaginary part of the DC bin, it must be cleared before computing the amplitudes"
        f[0].im = 0.0;

        // Add energy to spectrum
        for (v, s) in f.iter().zip(self.spec.iter_mut()) {
            let e = (v * v.conj()).re(); // energy: v * ~v = r^2 = |v|^2
            *s += e * self.scaling;
        }

        self.nseg += 1;
    }

    /// Compute Welch-spectrum (WARNING: does not reset).
    pub fn compute_spectrum(&mut self) -> [f32; NFFT / 2] {
        let mut spec = self.spec.clone().into_array::<{ NFFT / 2 }>().unwrap();

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
    pub fn take_spectrum(&mut self) -> [f32; NFFT / 2] {
        let spec = self.compute_spectrum();
        self.reset();

        spec
    }
}

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

    #[test]
    fn test_hanning_window() {
        for i in 0..NSEG {
            let w = hanning::hanning(i, NSEG);

            // pre-computed using np.hanning
            assert_abs_diff_eq!(w, hanning::COEFFS[i], epsilon = 0.000001);
        }

        // scaling
        let acorr: f32 = 4096.0 / hanning::COEFFS.iter().sum::<f32>();
        assert_abs_diff_eq!(acorr, hanning::ACORR, epsilon = 0.000001);
        assert_abs_diff_eq!(
            acorr,
            hanning::HANNING_AMPLITUDE_CORRECTION,
            epsilon = 0.000001
        );

        let ecorr: f32 = f32::sqrt(4096.0 / hanning::COEFFS.iter().map(|v| v * v).sum::<f32>());
        assert_abs_diff_eq!(ecorr, hanning::ECORR, epsilon = 0.000001);
        assert_abs_diff_eq!(
            ecorr,
            hanning::HANNING_ENERGY_CORRECTION,
            epsilon = 0.000001
        );
    }

    #[test]
    fn test_welch_synth1() {
        let mut w = Welch::new(26.);
        let mut data = npyz::npz::NpzArchive::open("tests/data/welch/welch_test_1.npz").unwrap();
        let s = data.by_name("s").unwrap().unwrap();
        for v in s.data::<f64>().unwrap() {
            w.sample(v.unwrap() as f32);
        }

        let spec = w.take_spectrum();
        println!("{:?}", spec);

        use std::fmt::Write;
        let mut str = std::string::String::new();
        writeln!(&mut str, "pxx = {:?}\n", spec).unwrap();

        std::fs::write("tests/data/welch/welch_test_1_rust_pxx", &str).unwrap();

        // use same welch instance again, to test if reset works.
        let mut data = npyz::npz::NpzArchive::open("tests/data/welch/welch_test_1.npz").unwrap();
        let s = data.by_name("s").unwrap().unwrap();
        for v in s.data::<f64>().unwrap() {
            w.sample(v.unwrap() as f32);
        }
        let spec2 = w.take_spectrum();

        assert_eq!(spec, spec2);
    }

    #[test]
    fn test_welch_rfftfreq() {
        let w = Welch::new(26.);
        let mut data = npyz::npz::NpzArchive::open("tests/data/welch/welch_test_1.npz").unwrap();
        let f = data.by_name("f").unwrap().unwrap();

        let rf = w.rfftfreq();

        assert_eq!(rf.len(), f.len() as usize - 1);

        for (ff, rff) in f.data::<f64>().unwrap().zip(&rf) {
            assert_abs_diff_eq!(ff.unwrap() as f32, rff);
        }
    }

    #[test]
    fn test_cut_offs() {
        let w = Welch::new(26.);
        let rf = w.rfftfreq();

        let i0 = rf.iter().copied().position(|f| f > f0).unwrap();
        let i1 = rf.iter().copied().position(|f| f > f1).unwrap();

        println!("i0 = {i0} => {}", rf[i0]);
        println!("i1 = {i1} => {}", rf[i1]);

        let N = i1 - i0;
        println!("bins: {N}");
    }
}
