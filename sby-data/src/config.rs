use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub address: SocketAddr,
    pub database: Option<PathBuf>,
}

impl Config {
    pub fn default() -> Config {
        Config {
            address: "0.0.0.0:3000".parse().unwrap(),
            database: None
        }
    }

    pub fn from_path<P: AsRef<Path>>(p: P) -> Config {
        let p = p.as_ref();

        let f = fs::read_to_string(p).expect("could not read config file");
        toml::from_str(&f).expect("could not parse config file")
    }
}
