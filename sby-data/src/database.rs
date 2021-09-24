use eyre::Result;
use std::path::{Path, PathBuf};

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
