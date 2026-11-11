use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub address: SocketAddr,
    pub database: Option<String>,
    pub tokens: Vec<String>,
    pub read_tokens: Vec<String>,
    pub files: Option<PathBuf>,
}

impl Config {
    pub fn default() -> Config {
        Config {
            address: "0.0.0.0:3000".parse().unwrap(),
            database: None,
            tokens: Vec::new(),
            read_tokens: Vec::new(),
            files: None,
        }
    }

    #[cfg(test)]
    pub fn test_config() -> Config {
        Config {
            address: "0.0.0.0:3000".parse().unwrap(),
            database: None,
            tokens: vec!["token1".into()],
            read_tokens: vec!["r-token1".into()],
            files: None,
        }
    }

    pub fn from_path<P: AsRef<Path>>(p: P) -> Config {
        let p = p.as_ref();

        let f = fs::read_to_string(p).expect("could not read config file");
        let c: Config = toml::from_str(&f).expect("could not parse config file");

        #[cfg(not(debug_assertions))]
        c.reject_default_tokens();

        c
    }

    /// Panic if any token is one of the example tokens shipped in `sfy-data.toml`.
    #[allow(dead_code)]
    fn reject_default_tokens(&self) {
        let defaults: Config =
            toml::from_str(include_str!("../sfy-data.toml")).expect("bad bundled config");

        let is_default =
            |t: &String| defaults.tokens.contains(t) || defaults.read_tokens.contains(t);

        if self
            .tokens
            .iter()
            .chain(self.read_tokens.iter())
            .any(is_default)
        {
            panic!("config uses the default tokens from sfy-data.toml, refusing to start: set your own tokens");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_default_conf() {
        let c = fs::read_to_string("sfy-data.toml").unwrap();
        let c: Config = toml::from_str(&c).unwrap();
        println!("{:#?}", c);
    }

    #[test]
    #[should_panic(expected = "default tokens")]
    fn reject_default_tokens() {
        let c = fs::read_to_string("sfy-data.toml").unwrap();
        let c: Config = toml::from_str(&c).unwrap();
        c.reject_default_tokens();
    }

    #[test]
    fn test_conf_not_default() {
        Config::test_config().reject_default_tokens();
    }

    #[test]
    fn load_test_conf() {
        let _c = Config::test_config();
    }
}
