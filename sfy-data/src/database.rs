use eyre::Result;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Database> {
        let path: PathBuf = path.as_ref().into();
        info!("opening database at: {:?}", path);

        ensure!(path.exists(), "database path does not exist");

        Ok(Database { path })
    }

    pub fn buoy<'db>(&'db mut self, dev: &str) -> eyre::Result<Buoy<'db>> {
        let path = self.path.join(dev);

        if !path.exists() {
            info!("creating dir for buoy: {}", dev);
            fs::create_dir_all(&path)?;
        }

        Ok(Buoy {
            name: String::from(dev),
            path,
            _db: &PhantomData,
        })
    }

    #[cfg(test)]
    pub fn temporary() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path()).unwrap();

        warn!("create temporary database at: {:?}", dir.path());

        (dir, db)
    }
}

#[derive(Debug)]
pub struct Buoy<'a> {
    name: String,
    path: PathBuf,
    _db: &'a PhantomData<()>,
}

impl<'a> Buoy<'a> {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn append(&mut self, file: impl AsRef<Path>, data: impl AsRef<[u8]>) -> eyre::Result<()> {
        use tempfile::NamedTempFile;
        use tokio::fs;

        let data = data.as_ref();
        let file = file.as_ref();

        debug!("buoy: {}: appending file: {:?}, size: {}", self.name, file, data.len());

        let path = self.path.join(file);

        ensure!(!path.exists(), "file already exists!");

        let tmp = NamedTempFile::new()?;
        fs::write(tmp.path(), data).await?;
        fs::rename(tmp.path(), path).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_temporary() {
        let (tmp, _db) = Database::temporary();

        assert!(tmp.path().exists());
    }

    #[test]
    fn get_new_buoy() {
        let (_tmp, mut db) = Database::temporary();
        let _b = db.buoy("test-01");
    }

    #[tokio::test]
    async fn add_some_entries() {
        let (_tmp, mut db) = Database::temporary();
        let mut b = db.buoy("buoy-01").unwrap();

        b.append("entry-0", "data-0").await.unwrap();
        b.append("entry-1", "data-1").await.unwrap();

        assert_eq!(
            fs::read(b.path().join("entry-0")).unwrap().as_slice(),
            b"data-0"
        );
    }

    #[tokio::test]
    async fn add_existing_entry() {
        let (_tmp, mut db) = Database::temporary();
        let mut b = db.buoy("buoy-01").unwrap();

        b.append("entry-0", "data-0").await.unwrap();
        assert!(b.append("entry-0", "data-1").await.is_err());
    }
}
