use core::simd::{f32x4, SimdFloat};
use heapless::Deque;

/// Sample rate.
pub const FREQ: f32 = 833.0; ///////////////////////////

pub mod hz50 {
    /// Filter order, length or number of taps.
    pub const NTAP: usize = 129;

    /// Filter coefficients. Generated with Pythons `scipy.signal.firwin(...)`.
    pub const COEFFS: [f32; NTAP] = include!("firwin.25_52_coeff");

    /// Cut-off frequency of filter.
    pub const CUTOFF: f32 = 26.0;

    // True cut-off frequency as generated with `firwin`. Must have some margin to sufficiently
    // attenuate frequencies close to Nyquist.
    // pub const TRUE_CUTOFF: f32 = 20.0;
}

pub mod hz20 {
    /// Filter order, length or number of taps.
    pub const NTAP: usize = 129;

    /// Filter coefficients. Generated with Pythons `scipy.signal.firwin(...)`.
    pub const COEFFS: [f32; NTAP] = include!("firwin.10_52_coeff");

    /// Cut-off frequency of filter.
    pub const CUTOFF: f32 = 13.0;

    // True cut-off frequency as generated with `firwin`. Must have some margin to sufficiently
    // attenuate frequencies close to Nyquist.
    // pub const TRUE_CUTOFF: f32 = 8.0;
}

#[cfg(feature = "20Hz")]
pub use hz20::*;

#[cfg(not(feature = "20Hz"))]
pub use hz50::*;

/// Maximum decimation given `CUTOFF` and sample rate (`FREQ`).
pub const DECIMATE: u8 = (FREQ / CUTOFF / 2.) as u8;

/// Output frequency after decimation.
pub const OUT_FREQ: f32 = FREQ / DECIMATE as f32;

/// The delay (in seconds) introduced by the filter: half the length of the filter.
pub const DELAY: f32 = (NTAP / 2) as f32 / FREQ;

/// A running FIR filter with pre-computed coefficients.
pub struct FIR {
    samples: Deque<f32, NTAP>,
}

impl FIR {
    pub fn new() -> FIR {
        let mut samples = Deque::new();

        while samples.push_back(0.0).is_ok() {}

        FIR { samples }
    }

    /// Update filter with new sample value, apply filter and output current filtered value.
    pub fn filter(&mut self, v: f32) -> f32 {
        self.put(v);
        self.value()
    }

    fn put(&mut self, v: f32) {
        self.samples.pop_front();
        self.samples.push_back(v).unwrap();
    }

    fn value(&self) -> f32 {
        // Convolve filter with samples.

        // self.samples
        //     .iter()
        //     .zip(&COEFFS)
        //     .fold(0.0, |a, (s, c)| a + (s * c))

        // debug_assert_eq!(self.samples.len() % 4, 0);
        // debug_assert_eq!(COEFFS.len() % 4, 0);
        debug_assert_eq!(COEFFS.len(), self.samples.len());

        let (f, b) = self.samples.as_slices();
        let (cf, cb) = COEFFS.split_at(f.len());

        debug_assert_eq!(f.len(), cf.len());
        debug_assert_eq!(b.len(), cb.len());

        // First half of dequeue
        let (p, m, s) = f.as_simd::<4>();
        let me = p.len() + m.len() * 4;
        let cp = &cf[..p.len()];
        let cm = cf[p.len()..me].array_chunks();
        let cs = &cf[me..];

        debug_assert_eq!(cp.len(), p.len());
        debug_assert_eq!(cm.len(), m.len());
        debug_assert_eq!(cs.len(), s.len());

        let sp = p.iter().zip(cp).fold(0.0, |a, (s, c)| a + (s * c));
        let ss = s.iter().zip(cs).fold(0.0, |a, (s, c)| a + (s * c));

        let fsums = f32x4::from_array([sp, 0.0, 0.0, ss]);
        let fsums = m
            .iter()
            .zip(cm)
            .fold(fsums, |a, (s, c)| a + (s * f32x4::from_array(*c)));

        // Second half of dequeue
        let (p, m, s) = b.as_simd::<4>();
        let me = p.len() + m.len() * 4;
        let cp = &cb[..p.len()];
        let cm = cb[p.len()..me].array_chunks();
        let cs = &cb[me..];
        debug_assert_eq!(cp.len(), p.len());
        debug_assert_eq!(cm.len(), m.len());
        debug_assert_eq!(cs.len(), s.len());

        let sp = p.iter().zip(cp).fold(0.0, |a, (s, c)| a + (s * c));
        let ss = s.iter().zip(cs).fold(0.0, |a, (s, c)| a + (s * c));

        let bsums = f32x4::from_array([sp, 0.0, 0.0, ss]);
        let bsums = m
            .iter()
            .zip(cm)
            .fold(bsums, |a, (s, c)| a + (s * f32x4::from_array(*c)));

        (fsums + bsums).reduce_sum()
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        while self.samples.push_back(0.0).is_ok() {}
    }

    pub fn into_decimator(self) -> Decimator {
        Decimator { fir: self, m: 0 }
    }
}

/// Wrapper around filter that only calculates filter output for
/// every M'th sample.
pub struct Decimator {
    fir: FIR,
    m: u8,
}

impl Decimator {
    /// Update filter with new sample. A filtered output value is calculated and returned
    /// _if_ `DECIMATE` samples has passed. Otherwise `None` is returned.
    pub fn decimate(&mut self, v: f32) -> Option<f32> {
        self.fir.put(v);

        if self.m % DECIMATE == 0 {
            self.m = 1;

            Some(self.fir.value())
        } else {
            self.m += 1;
            None
        }
    }

    pub fn reset(&mut self) {
        self.m = 0;
        self.fir.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn setup_filter() {
        let f = FIR::new();
        assert_eq!(f.samples.len(), NTAP);
    }

    #[test]
    fn add_some_filter() {
        let mut f = FIR::new();

        for v in 0..256 {
            f.filter(v as f32);
            assert_eq!(f.samples.len(), NTAP);
        }
        assert_eq!(f.samples.len(), NTAP);
    }

    #[test]
    fn zero() {
        let mut f = FIR::new();

        for _ in 0..256 {
            let o = f.filter(0.0);
            assert_eq!(o, 0.0);
            assert_eq!(f.samples.len(), NTAP);
        }
        assert_eq!(f.samples.len(), NTAP);
    }

    #[test]
    fn reset() {
        let mut f = FIR::new();
        assert_eq!(f.samples.len(), NTAP);

        for _ in 0..256 {
            let o = f.filter(1.0);
            assert_ne!(o, 0.0);
            assert_eq!(f.samples.len(), NTAP);
        }

        f.reset();
        assert_eq!(f.samples.len(), NTAP);
        let o = f.filter(0.0);
        assert_eq!(o, 0.0);
        assert_eq!(f.samples.len(), NTAP);
    }

    #[test]
    fn sin_within_cutoff() {
        let mut f = FIR::new();

        let fs = FREQ;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        let sf = s.iter().map(|s| f.filter(*s)).collect::<Vec<_>>();

        println!("sf: {:?}", sf);
        for (s, sf) in s.iter().zip(sf.iter().skip(128 / 2)).skip(128) {
            let diff = (s - sf).abs();
            println!("diff: {}", diff);
            assert!(diff < 0.02);
        }
    }

    #[test]
    fn sin_outside_cutoff() {
        let mut f = FIR::new();

        let fs = 208.;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        let sf = s.iter().map(|s| f.filter(*s)).collect::<Vec<_>>();

        println!("sf: {:?}", sf);
        for (s, sf) in s.iter().zip(sf.iter().skip(128 / 2)).skip(128) {
            let diff = (s - sf).abs();
            println!("diff: {}", diff);
            assert!(diff < 0.02);
        }
    }

    #[test]
    fn decimate() {
        let mut f = FIR::new();
        let mut d = FIR::new().into_decimator();

        let fs = FREQ;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        println!("decimate: {}", DECIMATE);
        println!("out_freq: {}", OUT_FREQ);

        let sf = s
            .iter()
            .map(|s| f.filter(*s))
            .step_by(DECIMATE as usize)
            .collect::<Vec<_>>();
        let df = s.iter().filter_map(|s| d.decimate(*s)).collect::<Vec<_>>();
        assert_eq!(sf, df);
        assert_eq!(df.len(), 4096 / DECIMATE as usize);
    }

    #[bench]
    fn decimate_cycle(b: &mut Bencher) {
        let mut d = FIR::new().into_decimator();
        let fs = FREQ;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        let mut is = s.iter().cycle();

        b.iter(|| {
            test::black_box(d.decimate(*is.next().unwrap()));
        });
    }

    #[bench]
    fn decimate_many(b: &mut Bencher) {
        let mut d = FIR::new().into_decimator();
        let fs = FREQ;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        b.iter(|| {
            for v in &s {
                test::black_box(d.decimate(*v));
            }
        });
    }

    #[bench]
    fn fir_cycle(b: &mut Bencher) {
        let mut f = FIR::new();
        let fs = FREQ;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        let mut is = s.iter().cycle();

        b.iter(|| {
            test::black_box(f.filter(*is.next().unwrap()));
        });
    }
}
