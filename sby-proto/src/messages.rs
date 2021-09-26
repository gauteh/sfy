//! The protocol.
//!
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Drifter {
    pub id: heapless::String<8>,
    // pub address: IpAddr, maybe use embedded-nal?
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Messages {
    GPS(heapless::String<100>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_messages() {
    }
}

