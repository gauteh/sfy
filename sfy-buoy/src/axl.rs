use defmt::{write, Format, Formatter};
use heapless::Vec;

#[cfg(feature = "raw")]
pub const SAMPLE_NO: usize = 1024;

#[cfg(not(feature = "raw"))]
pub const SAMPLE_NO: usize = 1024;

pub const SAMPLE_SZ: usize = 3;
pub const AXL_SZ: usize = SAMPLE_SZ * SAMPLE_NO;
pub const VERSION: u32 = 5;

/// Maximum length of base64 string from [f16; AXL_SZ]
pub const AXL_OUTN: usize = { AXL_SZ * 2 } * 4 / 3 + 4;

/// Max size of `AxlPacket` serialized using postcard with COBS. Set with some margin since
/// postcard messages are not fixed size.
#[cfg(feature = "raw")]
pub const AXL_POSTCARD_SZ: usize = 1024 * 10;

#[cfg(not(feature = "raw"))]
pub const AXL_POSTCARD_SZ: usize = 1024 * 10;

#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AxlPacket {
    /// Timestamp of sample at `offset` in ms.
    pub timestamp: i64,

    /// Offset in IMU FIFO at time of timestamp.
    pub offset: u16,

    /// ID on SD-card. This one is not necessarily unique. Will not be set
    /// before package has been written to SD-card.
    pub storage_id: Option<u32>,
    pub storage_version: u32,

    /// Time of position in seconds.
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,
    pub temperature: f32,

    /// Frequency of data.
    pub freq: f32,

    /// IMU data. This is moved to the payload when transmitting.
    pub data: Vec<u16, { AXL_SZ }>,
}

fn f32_not_normal(f: &f32) -> bool {
    !f32::is_subnormal(*f)
}

#[derive(serde::Serialize, Default)]
pub struct AxlPacketMeta {
    pub timestamp: i64,
    pub offset: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_id: Option<u32>,
    pub storage_version: u32,

    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,

    #[serde(skip_serializing_if = "f32_not_normal")]
    pub temperature: f32,

    pub freq: f32,
    pub length: u32,
}

impl core::fmt::Debug for AxlPacket {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(fmt, "AxlPacket(timestamp: {}, offset: {}, storage_id: {:?} (v: {:?}), position_time: {}, lon: {}, lat: {}, temp: {}, freq: {}, data (length): {}))",
            self.timestamp,
            self.offset,
            self.storage_id,
            self.storage_version,
            self.position_time,
            self.lon,
            self.lat,
            self.temperature,
            self.freq,
            self.data.len()
            )
    }
}

impl Format for AxlPacket {
    fn format(&self, fmt: Formatter) {
        write!(fmt, "AxlPacket(timestamp: {}, offset: {}, storage_id: {:?}, position_time: {}, lon: {}, lat: {}, temp: {}, freq: {}, data (length): {}))",
            self.timestamp,
            self.offset,
            self.storage_id,
            self.position_time,
            self.lon,
            self.lat,
            self.temperature,
            self.freq,
            self.data.len()
            );
    }
}

impl AxlPacket {
    pub fn base64(&self) -> Vec<u8, AXL_OUTN> {
        let mut b64: Vec<_, AXL_OUTN> = Vec::new();
        b64.resize_default(AXL_OUTN).unwrap();

        // Check endianness (TODO:  swap order if compiled for big endian machine).
        #[cfg(target_endian = "big")]
        compile_error!("serializied samples are assumed to be in little endian, target platform is big endian and no conversion is implemented.");

        let data = bytemuck::cast_slice(&self.data);
        let written = base64::encode_config_slice(data, base64::STANDARD, &mut b64);
        b64.truncate(written);

        b64
    }

    /// Split package into metadata and payload.
    pub fn split(&self) -> (AxlPacketMeta, Vec<u8, AXL_OUTN>) {
        let b64 = self.base64();

        let meta = AxlPacketMeta {
            timestamp: self.timestamp,
            offset: self.offset as u32,
            length: b64.len() as u32,
            freq: self.freq,
            storage_id: self.storage_id,
            storage_version: self.storage_version,
            position_time: self.position_time,
            lon: self.lon,
            lat: self.lat,
            temperature: self.temperature,
        };


        (meta, b64)
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
            storage_version: VERSION,
            temperature: 0.0,
            data: (0..AXL_SZ)
                .map(|v| v as u16)
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }

    #[test]
    fn postcard_size() {
        let p = AxlPacket {
            timestamp: 100212312312330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 0,
            storage_id: Some(1489),
            storage_version: VERSION,
            temperature: 0.0,
            data: (0..AXL_SZ)
                .map(|v| v as u16)
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        assert!(p.data.is_full());

        let v: Vec<_, { AXL_POSTCARD_SZ }> = postcard::to_vec_cobs(&p).unwrap();
        println!("{}", v.len());

        assert!(v.len() < AXL_POSTCARD_SZ);

        // This does not include the additional size used by COBS.
        // assert!(AXL_POSTCARD_SZ >= AxlPacket::POSTCARD_MAX_SIZE);
    }
}
