use eyre::Result;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Database> {
        let path: PathBuf = path.as_ref().into();

        ensure!(path.exists(), "datbase path does not exist");


        Ok(Database { path })
    }

    pub fn temporary() -> Result<Database> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_temporary() {
        Database::temporary().unwrap();
    }

    #[test]
    fn open_db() {
        Database::open(".").unwrap();
    }
}
