use std::path::{Path, PathBuf};
use std::str::FromStr;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json as json;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

#[derive(Debug)]
pub struct Database {
    db: SqlitePool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageInfo {
    pub current_id: Option<u64>,
    pub request_start: Option<u64>,
    pub request_end: Option<u64>,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Database> {
        let path: PathBuf = path.as_ref().into();
        info!("opening database at: {:?}", path);

        let db = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.to_string_lossy()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .page_size(10 * 1024)
            .pragma("mmap_size", "3000000000")
            .pragma("temp_store", "memory");
        let db = SqlitePoolOptions::new().connect_with(db).await?;

        info!("running db migrations..");
        sqlx::migrate!("./migrations").run(&db).await?;

        Ok(Database { db })
    }

    /// Open buoy for writing.
    pub async fn buoy(&self, dev: &str) -> eyre::Result<Buoy> {
        let buoy = sqlx::query!("SELECT dev, name FROM buoys where dev = ?1", dev)
            .fetch_optional(&self.db)
            .await?;

        let known = buoy.is_some();

        if !known {
            debug!("Unknown buoy: {}", dev);
        }

        let name = buoy.map(|b| b.name).flatten();

        Ok(Buoy {
            dev: String::from(dev),
            known,
            name,
            db: self.db.clone().clone(),
        })
    }

    /// Get list of buoys.
    pub async fn buoys(&self) -> eyre::Result<Vec<(String, String)>> {
        let buoys = sqlx::query!("SELECT dev, name FROM buoys ORDER BY dev")
            .fetch_all(&self.db)
            .await?
            .iter()
            .map(|r| {
                (
                    r.dev.clone().unwrap_or(String::new()),
                    r.name.clone().unwrap_or(String::new()),
                )
            })
            .collect();

        Ok(buoys)
    }

    #[cfg(test)]
    pub async fn temporary() -> Database {
        warn!("create temporary database at in memory");

        Database::open(":memory:").await.unwrap()
    }
}

#[derive(Debug)]
pub struct Buoy {
    dev: String,
    /// Does the buoy exist in the database already.
    known: bool,
    name: Option<String>,
    db: SqlitePool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub received: i64,
    pub event: String,
    pub data: Option<Vec<u8>>,
}

impl Buoy {
    /// Append new event to buoy, `name` is parsed serial number of buoy.
    pub async fn append(
        &mut self,
        name: Option<String>,
        event: impl AsRef<Path>,
        received: u64,
        data: impl AsRef<[u8]>,
    ) -> eyre::Result<()> {
        let data = data.as_ref();
        let event = event.as_ref().to_string_lossy().into_owned();

        if let Some(ref name) = name {
            if self.name.as_ref() != Some(&name) {
                debug!("Updating name for: {} to {}", self.dev, name);
                self.known = false;
            }
        }

        if !self.known {
            info!(
                "Updating or inserting buoy from {:?} to {:?}",
                self.name, name
            );

            sqlx::query!(
                "INSERT OR REPLACE INTO buoys (dev, name) VALUES ( ?1, ?2 )",
                self.dev,
                name
            )
            .execute(&self.db)
            .await?;

            self.known = true;
        }

        debug!(
            "buoy: {} ({:?}): appending event: {:?}, received: {}, size: {}",
            self.dev,
            self.name,
            event,
            received,
            data.len()
        );

        let r = received as i64;
        sqlx::query!(
            "INSERT INTO events (dev, received, event, data) VALUES ( ?1, ?2, ?3, ?4 )",
            self.dev,
            r,
            event,
            data
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn entries(&self) -> Result<Vec<String>> {
        ensure!(self.known, "No such buoy");

        let events = sqlx::query!(
            "SELECT received, event FROM events where dev = ?1 ORDER BY received",
            self.dev
        )
        .map(|r| format!("{}-{}", r.received, r.event))
        .fetch_all(&self.db)
        .await?;

        Ok(events)
    }

    /// Get the last received axl.qo entry for the buoy.
    pub async fn last(&self) -> Result<Vec<u8>> {
        ensure!(self.known, "No such buoy");

        let event = sqlx::query!("SELECT data FROM events WHERE dev = ?1 AND instr(event, 'axl.qo') ORDER BY received DESC LIMIT 1", self.dev)
            .fetch_one(&self.db)
            .await?;

        match event.data {
            Some(event) => Ok(event),
            None => Err(eyre!("No axl entry found.")),
        }
    }

    pub async fn storage_info(&self) -> Result<StorageInfo> {
        ensure!(self.known, "No such buoy");

        let event = sqlx::query!("SELECT data FROM events WHERE dev = ?1 AND instr(event, 'storage.db') ORDER BY received DESC LIMIT 1", self.dev)
            .fetch_one(&self.db)
            .await?;

        match &event.data {
            Some(event) => {
                let body: json::Value = json::from_slice(&event)?;

                let info = body
                    .get("body")
                    .ok_or(eyre!("no event field"))?;

                let current_id = info.get("current_id").and_then(json::Value::as_u64);
                let request_start = info.get("request_start").and_then(json::Value::as_u64);
                let request_end = info.get("request_end").and_then(json::Value::as_u64);

                Ok(StorageInfo { current_id, request_start, request_end })
            },
            None => Err(eyre!("No storage entry found."))
        }
    }

    pub async fn get(&self, file: impl AsRef<Path>) -> Result<Vec<u8>> {
        ensure!(self.known, "No such buoy");

        let file = file.as_ref().to_string_lossy().into_owned();

        let (received, file) = file
            .split_once('-')
            .ok_or(eyre!("incorrect format of event"))?;

        let event = sqlx::query!(
            "SELECT data FROM events WHERE dev = ?1 AND received = ?2 AND event = ?3",
            self.dev,
            received,
            file
        )
        .fetch_one(&self.db)
        .await?;

        match event.data {
            Some(event) => Ok(event),
            None => Err(eyre!("No such event found.")),
        }
    }

    pub async fn list_range(&self, start: i64, end: i64) -> Result<Vec<(i64, String)>> {
        ensure!(self.known, "No such buoy");

        let events = sqlx::query!(
            "SELECT event, received FROM events WHERE dev = ?1 AND received >= ?2 AND received <= ?3 ORDER BY received",
            self.dev,
            start,
            end,
        )
        .map(|r| (r.received, r.event))
        .fetch_all(&self.db)
        .await?;

        Ok(events)
    }

    pub async fn get_range(&self, start: i64, end: i64) -> Result<Vec<Event>> {
        ensure!(self.known, "No such buoy");

        let events = sqlx::query_as!(
            Event,
            "SELECT event, received, data FROM events WHERE dev = ?1 AND received >= ?2 AND received <= ?3 ORDER BY received",
            self.dev,
            start,
            end,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_temporary() {
        let _db = Database::temporary().await;
    }

    #[tokio::test]
    async fn get_new_buoy() {
        let db = Database::temporary().await;
        let _b = db.buoy("test-01").await;
    }

    #[tokio::test]
    async fn add_some_entries() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();

        b.append(None, "entry-0", 0, "data-0").await.unwrap();
        b.append(None, "entry-1", 0, "data-1").await.unwrap();

        assert_eq!(b.get("0-entry-0").await.unwrap(), b"data-0");
    }

    #[tokio::test]
    async fn add_existing_entry() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();

        b.append(None, "entry-0", 0, "data-0").await.unwrap();
        assert!(b.append(None, "entry-0", 0, "data-1").await.is_err());
    }

    #[tokio::test]
    async fn list_buoys() {
        let db = Database::temporary().await;

        let mut b = db.buoy("buoy-01").await.unwrap();
        b.append(None, "entry-0", 0, "data-0").await.unwrap();

        let mut b = db.buoy("buoy-02").await.unwrap();
        b.append(None, "entry-1", 0, "data-1").await.unwrap();

        let devs = db.buoys().await.unwrap();
        let devs: Vec<_> = devs.iter().map(|(dev, _)| dev).collect();

        assert_eq!(devs, ["buoy-01", "buoy-02"]);
    }

    #[tokio::test]
    async fn list_entries() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();
        b.append(None, "entry-0", 0, "data-0").await.unwrap();
        b.append(None, "entry-1", 0, "data-1").await.unwrap();

        assert_eq!(
            db.buoy("buoy-01").await.unwrap().entries().await.unwrap(),
            ["0-entry-0", "0-entry-1"]
        );
    }

    #[tokio::test]
    async fn append_get() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();
        b.append(None, "entry-0", 0, "data-0").await.unwrap();

        assert_eq!(b.get("0-entry-0").await.unwrap(), b"data-0");
    }

    #[tokio::test]
    async fn append_get_range() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();
        b.append(None, "entry-0", 0, "data-0").await.unwrap();
        b.append(None, "entry-1", 1, "data-1").await.unwrap();
        b.append(None, "entry-2", 2, "data-2").await.unwrap();
        b.append(None, "entry-3", 3, "data-3").await.unwrap();

        assert_eq!(b.get("0-entry-0").await.unwrap(), b"data-0");

        let r1 = b.get_range(0, 2).await.unwrap();
        assert_eq!(r1.len(), 3);

        let Event {
            received,
            event,
            data,
        } = &r1[0];
        assert_eq!(*received, 0);
        assert_eq!(event, "entry-0");
        assert_eq!(data, &Some(b"data-0".to_vec()));

        let Event {
            received,
            event,
            data,
        } = &r1[2];
        assert_eq!(*received, 2);
        assert_eq!(event, "entry-2");
        assert_eq!(data, &Some(b"data-2".to_vec()));

        let r2 = b.get_range(2, 3).await.unwrap();
        assert_eq!(r2.len(), 2);

        let Event {
            received,
            event,
            data,
        } = &r2[0];
        assert_eq!(*received, 2);
        assert_eq!(event, "entry-2");
        assert_eq!(data, &Some(b"data-2".to_vec()));

        let Event {
            received,
            event,
            data,
        } = &r2[1];
        assert_eq!(*received, 3);
        assert_eq!(event, "entry-3");
        assert_eq!(data, &Some(b"data-3".to_vec()));
    }

    #[tokio::test]
    async fn append_last() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();
        b.append(None, "entry-0-axl.qo", 0, "data-0").await.unwrap();
        b.append(None, "entry-1-sessi.qo", 0, "data-1")
            .await
            .unwrap();

        assert_eq!(b.last().await.unwrap(), b"data-0");
    }

    #[tokio::test]
    async fn append_get_storage_info() {
        let db = Database::temporary().await;
        let mut b = db.buoy("buoy-01").await.unwrap();

        let data = std::fs::read("tests/events/1653994017660-ae50c1e9-0800-4fd9-9cb6-cdd6a6d08eb3_storage.db.json").unwrap();

        b.append(None, "ae50c1e9-0800-4fd9-9cb6-cdd6a6d08eb3_storage.db", 1653994017660, &data).await.unwrap();

        let info = b.storage_info().await.unwrap();
        println!("{:?}", info);

        assert_eq!(info.current_id, Some(40002));
        assert_eq!(info.request_start, None);
        assert_eq!(info.request_end, None);
    }
}
