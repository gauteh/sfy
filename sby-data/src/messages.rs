//! The protocol.
//!
//! probably use `postcard` for serialization. no_std made for embedded stuff.
//! this file should go in a separate crate, and stay no_std.
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize)]
pub struct Drifter {
    pub id: String,
    pub address: IpAddr,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Messages {
    GPS(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_messages() {
    }
}
