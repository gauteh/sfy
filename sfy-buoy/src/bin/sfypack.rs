use argh::FromArgs;
use chrono::NaiveDateTime;
use serde_json as json;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use sfy::axl;

#[derive(FromArgs)]
/// Load and print Axl package from binary collection.
struct SfyPack {
    #[argh(positional, description = "file name")]
    file: PathBuf,

    #[argh(switch, short = 'l', description = "list packages")]
    list: bool,

    #[argh(switch, description = "export to JSON")]
    json: bool,

    #[argh(switch, description = "simulate a note.add event")]
    note: bool,
}

fn main() -> anyhow::Result<()> {
    let pck: SfyPack = argh::from_env();
    eprintln!("Loading collection from: {:?}", pck.file);

    let c = Collection::from_file(&pck.file)?;
    eprintln!("Loaded {} packages.", c.len());

    if pck.list {
        for p in c.iter() {
            let ts = NaiveDateTime::from_timestamp(
                p.timestamp / 1000,
                (p.timestamp % 1000 * 1_000_000).try_into().unwrap(),
            );
            eprintln!("{:?}: {:?}", ts, p);
        }

        eprintln!("Listed {} packages.", c.len());
    }

    match (pck.json, pck.note) {
        (true, false) => {
            println!("{}", json::to_string_pretty(&c.pcks).unwrap());
        }
        (false, true) => {
            let pcks = c.pcks.iter().map(AxlNote::from).collect::<Vec<_>>();
            println!("{}", json::to_string_pretty(&pcks).unwrap());
        }
        (false, false) => (),
        _ => eprintln!("only one of --json and --note may be specified at the same time"),
    }

    Ok(())
}

/// Simulated note event
#[derive(serde::Serialize)]
pub struct AxlNote {
    body: axl::AxlPacketMeta,
    payload: String,
}

impl AxlNote {
    pub fn from(pck: &axl::AxlPacket) -> AxlNote {
        let b64 = pck.base64();

        let body = axl::AxlPacketMeta {
            timestamp: pck.timestamp,
            offset: pck.offset as u32,
            length: b64.len() as u32,
            freq: pck.freq,
            storage_id: pck.storage_id,
            position_time: pck.position_time,
            lon: pck.lon,
            lat: pck.lat,
        };

        let payload = String::from_utf8(b64.as_slice().to_vec()).unwrap();

        AxlNote { body, payload }
    }
}

struct Collection {
    pub pcks: Vec<axl::AxlPacket>,
}

impl Collection {
    pub fn from_file(p: impl AsRef<Path>) -> anyhow::Result<Collection> {
        let p = p.as_ref();
        let mut b = std::fs::read(p)?;

        if (b.len() % axl::AXL_POSTCARD_SZ) != 0 {
            eprintln!("Warning, collection consists of non-integer number of packages.");
        }

        let n = b.len() / axl::AXL_POSTCARD_SZ;

        eprintln!(
            "Parsing {} bytes of packages into {} packages..",
            b.len(),
            n
        );
        let pcks = b
            .chunks_exact_mut(axl::AXL_POSTCARD_SZ)
            .filter_map(|p| match postcard::from_bytes_cobs(p) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("failed to parse package: {:?}", e);
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(Collection { pcks })
    }
}

impl Deref for Collection {
    type Target = Vec<axl::AxlPacket>;

    fn deref(&self) -> &Vec<axl::AxlPacket> {
        &self.pcks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_collection() {
        let c = Collection::from_file("tests/data/73.1").unwrap();
        println!("packages: {}", c.pcks.len());

        let c = Collection::from_file("tests/data/74.1").unwrap();
        println!("packages: {}", c.pcks.len());

        // for p in c.pcks {
        //     println!("Package: {:?}", p);
        // }
    }
}
