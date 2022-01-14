use eyre::Result;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

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

    /// Open buoy for writing.
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

    /// Get list of buoys.
    pub async fn buoys(&self) -> eyre::Result<Vec<String>> {
        use tokio::fs;

        let mut entries = fs::read_dir(&self.path).await?;
        let mut buoys = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                buoys.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        Ok(buoys)
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

    pub fn tempfile(&mut self) -> eyre::Result<NamedTempFile> {
        let tmpdir = self.path.join("tmp");

        if !tmpdir.exists() {
            info!("creating temp dir in: {:?}", &self.path);
            fs::create_dir_all(&tmpdir)?;
        }

        Ok(NamedTempFile::new_in(tmpdir)?)
    }

    pub async fn append(&mut self, file: impl AsRef<Path>, data: impl AsRef<[u8]>) -> eyre::Result<()> {
        use tokio::fs;

        let data = data.as_ref();
        let file = file.as_ref();

        debug!("buoy: {}: appending file: {:?}, size: {}", self.name, file, data.len());

        let path = self.path.join(file);

        ensure!(!path.exists(), "file already exists!");

        let tmp = self.tempfile()?;
        fs::write(tmp.path(), data).await?;
        tmp.persist(path)?;

        Ok(())
    }

    pub async fn entries(&self) -> Result<Vec<String>> {
        use tokio::fs;

        let mut files = fs::read_dir(&self.path).await?;
        let mut entries = Vec::new();

        while let Some(file) = files.next_entry().await? {
            if file.file_type().await?.is_file() {
                entries.push(file.file_name().to_string_lossy().to_string());
            }
        }

        Ok(entries)
    }

    pub async fn get(&self, file: impl AsRef<Path>) -> Result<Vec<u8>> {
        use tokio::fs;

        let path = self.path.join(file);
        Ok(fs::read(path).await?)
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

    #[tokio::test]
    async fn list_buoys() {
        let (_tmp, mut db) = Database::temporary();
        db.buoy("buoy-01").unwrap();
        db.buoy("buoy-02").unwrap();

        assert_eq!(db.buoys().await.unwrap(), ["buoy-01", "buoy-02"]);
    }

    #[tokio::test]
    async fn list_entries() {
        let (_tmp, mut db) = Database::temporary();
        let mut b = db.buoy("buoy-01").unwrap();
        b.append("entry-0", "data-0").await.unwrap();
        b.append("entry-1", "data-1").await.unwrap();

        assert_eq!(db.buoy("buoy-01").unwrap().entries().await.unwrap(), ["entry-1", "entry-0"]);
    }

    #[tokio::test]
    async fn append_get() {
        let (_tmp, mut db) = Database::temporary();
        let mut b = db.buoy("buoy-01").unwrap();
        b.append("entry-0", "data-0").await.unwrap();

        assert_eq!(b.get("entry-0").await.unwrap(), b"data-0");
    }
}
