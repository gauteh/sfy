use heapless::Deque;

const NTAP: usize = 128;
const COEFFS: [f32; NTAP] = include!("firwin.coeff");
#[allow(dead_code)]
const FREQ: f32 = 833.0;
#[allow(dead_code)]
const CUTOFF: f32 = 25.0;
const DECIMATE: u16 = 25;

/// A running FIR filter with pre-computed coefficients.
pub struct FIR {
    samples: Deque<f32, NTAP>,
}

impl FIR {
    pub fn new() -> FIR {
        let mut samples = Deque::new();

        while let Ok(_) = samples.push_back(0.0) {}

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
        // Convolute filter with samples.
        self.samples
            .iter()
            .zip(&COEFFS)
            .fold(0.0, |a, (s, c)| a + (s * c))
    }

    pub fn into_decimator(self) -> Decimator {
        Decimator { fir: self, m: 0 }
    }
}

/// Wrapper around filter that only calculates filter output for
/// every M'th sample.
pub struct Decimator {
    fir: FIR,
    m: u16,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_filter() {
        let _f = FIR::new();
    }

    #[test]
    fn add_some_filter() {
        let mut f = FIR::new();

        for v in 0..256 {
            f.filter(v as f32);
        }
    }

    #[test]
    fn zero() {
        let mut f = FIR::new();

        for _ in 0..256 {
            let o = f.filter(0.0);
            assert_eq!(o, 0.0);
        }
    }

    #[test]
    fn sin_within_cutoff() {
        let mut f = FIR::new();

        let fs = 833.;
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

        let fs = 833.;
        let dt = 1. / fs;

        let t = (0..4096).map(|i| i as f32 * dt).collect::<Vec<_>>();
        let s = t
            .iter()
            .map(|t| 2. * (2. * t * 2. * std::f32::consts::PI).sin())
            .collect::<Vec<_>>();

        let sf = s
            .iter()
            .map(|s| f.filter(*s))
            .step_by(25)
            .collect::<Vec<_>>();
        let df = s.iter().filter_map(|s| d.decimate(*s)).collect::<Vec<_>>();
        assert_eq!(sf, df);
        assert_eq!(df.len(), 4096 / 25 + 1);
    }
}
