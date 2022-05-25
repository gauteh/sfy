use half::f16;

pub const SAMPLE_SZ: usize = 3;
pub const AXL_SZ: usize = SAMPLE_SZ * 1024;

/// Maximum length of base64 string from [f16; AXL_SZ]
pub const AXL_OUTN: usize = { AXL_SZ * 2 } * 4 / 3 + 4;

pub struct AxlPacket {
    /// Timstamp of sample at `offset`.
    pub timestamp: i64,
    pub offset: u16,
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,
    pub freq: f32,

    /// This is moved to the payload of the note.
    pub data: heapless::Vec<f16, { AXL_SZ }>,
}

impl AxlPacket {
    pub fn base64(&self) -> heapless::Vec<u8, AXL_OUTN> {
        let mut b64: heapless::Vec<_, AXL_OUTN> = heapless::Vec::new();
        b64.resize_default(AXL_OUTN).unwrap();

        // Check endianness (TODO: use byteorder or impl in hidefix to swap order if compiled for
        // big endian machine).
        #[cfg(target_endian = "big")]
        compile_error!("serializied samples are assumed to be in little endian, target platform is big endian and no conversion is implemented.");

        let data = bytemuck::cast_slice(&self.data);
        let written = base64::encode_config_slice(data, base64::STANDARD, &mut b64);
        b64.truncate(written);

        b64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_data_package() {
        let p = AxlPacket {
            timestamp: 0,
            position_time: 0,
            lat: 0.0,
            lon: 0.0,
            freq: 100.0,
            offset: 0,
            data: (0..3072)
                .map(|v| f16::from_f32(v as f32))
                .collect::<heapless::Vec<_, { AXL_SZ }>>(),
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }
}
