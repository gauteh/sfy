//! The protocol.
//!
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Drifter {
    pub id: heapless::String<8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Messages {
    GPS(heapless::String<100>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json as json;

    #[test]
    fn serialize_messages() {
        let g = Messages::GPS(heapless::String::from("sadf"));
        println!("{}", json::to_string_pretty(&g).unwrap());
    }
}

