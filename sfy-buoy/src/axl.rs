use heapless::Vec;
use half::f16;

pub const SAMPLE_SZ: usize = 3;
pub const AXL_SZ: usize = SAMPLE_SZ * 1024;

/// Maximum length of base64 string from [f16; AXL_SZ]
pub const AXL_OUTN: usize = { AXL_SZ * 2 } * 4 / 3 + 4;

/// Size of `AxlPacket` serialized using postcard with COBS. Experimentally derived.
pub const AXL_POSTCARD_SZ: usize = 6207;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct AxlPacket {
    /// Timstamp of sample at `offset`.
    pub timestamp: i64,

    /// Offset in IMU FIFO at time of timestamp.
    pub offset: u16,

    /// ID on SD-card. This one is not necessarily unique. Will not be set
    /// before package has been written to SD-card.
    pub storage_id: Option<u32>,

    /// Time of position.
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,

    /// Frequency of data.
    pub freq: f32,

    /// IMU data. This is moved to the payload when transmitting.
    pub data: Vec<f16, { AXL_SZ }>,
}

impl AxlPacket {
    pub fn base64(&self) -> Vec<u8, AXL_OUTN> {
        let mut b64: Vec<_, AXL_OUTN> = Vec::new();
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
            storage_id: Some(0),
            data: (0..3072)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }

    #[test]
    fn postcard_size() {
        let p = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: Some(1489),
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        let v: Vec<_, { AXL_OUTN }> = postcard::to_vec_cobs(&p).unwrap();
        println!("{}", v.len());

        assert_eq!(v.len(), AXL_POSTCARD_SZ);
    }
}
