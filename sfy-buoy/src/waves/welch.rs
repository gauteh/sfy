use heapless::Vec;

pub const NFFT: usize = 4096;
pub const NSEG: usize = 4096;
pub const NOVERLAP: usize = NSEG / 2;

// Cut-off frequencies for spectrum.
pub const f0: f32 = 0.01; // Hz
pub const f1: f32 = 5.0; // Hz

pub const WELCH_PACKET_SZ: usize = 124;

/// Maximum length of base64 string
pub const WELCH_OUTN: usize = { 6 * WELCH_PACKET_SZ * 2 } * 4 / 3 + 4;

pub struct Welch {
    buf: Vec<f64, NSEG>,
    spec: Vec<f64, NFFT>,
    nseg: u16,
}

impl Welch {
    pub fn new() {
        Welch {
            buf: Vec::new(),
            spec: Vec::new(),
            nseg: 0,
        }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.spec.clear();
    }

    /// Add new sample to buf: returns a spectrum if complete.
    pub fn sample(&mut self, z: f32) -> Option<Vec<NFFT>> {
        self.buf.push(z);

        if self.buf.is_full() {
            self.pop_buf();

            self.sample(z)
        } else {
            None
        }
    }

    /// Compute FFT of segment and merge with spectrum. Returns a spectrum if complete.
    pub fn pop_buf(&mut self) -> Option<Vec<NFFT>> {
        // Compute FFT from buf
        // Add to spec
        self.nseg += 1;
        // If spec is full, return spec and reset.
        if self.spec.is_full() {
            Some(self.pop_spec())
        } else {
            None
        }
    }

    /// Compute Welch-spectrum.
    pub fn pop_spec(&mut self) -> Vec<NFFT> {
        let spec = self.spec.clone();
        self.reset();

        // XXX: divide by self.nseg (if > 0)
        unimplemented!();

        spec
    }
}
