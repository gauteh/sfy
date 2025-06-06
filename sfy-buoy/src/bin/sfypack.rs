use argh::FromArgs;
use chrono::DateTime;
use serde_json as json;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use sfy::axl;
use sfy::storage::PACKAGE_SZ as RAW_PACKAGE_SZ;
use sfy::waves::VecRawAxl;

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

    #[argh(switch, description = "input file with raw-data")]
    raw: bool,
}

fn main() -> anyhow::Result<()> {
    let pck: SfyPack = argh::from_env();
    eprintln!("Loading collection from: {:?}", pck.file);

    let c = match pck.raw {
        false => Collection::from_file(&pck.file),
        true => Collection::from_file_raw(&pck.file),
    }?;
    eprintln!("Loaded {} packages.", c.len());

    if pck.list {
        for p in c.iter() {
            let ts = DateTime::from_timestamp(
                p.timestamp / 1000,
                (p.timestamp % 1000 * 1_000_000).try_into().unwrap(),
            )
            .unwrap();
            eprintln!("{:?}: {:?}", ts, p);
        }

        eprintln!("Listed {} packages.", c.len());
    }

    match (pck.json, pck.note) {
        (true, false) => {
            if let Some(raw) = &c.raw {
                println!("{}", json::to_string_pretty(&(&c, &raw)).unwrap());
            } else {
                println!("{}", json::to_string_pretty(&c).unwrap());
            }
        }
        (false, true) => {
            let pcks = if let Some(raw) = c.raw {
                c.pcks
                    .iter()
                    .zip(raw)
                    .map(|(p, r)| AxlNote::from(p, Some(r)))
                    .collect::<Vec<AxlNote>>()
            } else {
                c.pcks
                    .iter()
                    .map(|p| AxlNote::from(p, None))
                    .collect::<Vec<AxlNote>>()
            };
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
    raw: Option<Vec<f32>>,
}

impl AxlNote {
    pub fn from(pck: &axl::AxlPacket, raw: Option<Vec<f32>>) -> AxlNote {
        let (body, b64) = pck.split();

        let payload = String::from_utf8(b64.as_slice().to_vec()).unwrap();

        AxlNote { body, payload, raw }
    }
}

#[derive(serde::Serialize)]
struct Collection {
    pub pcks: Vec<axl::AxlPacket>,
    pub raw: Option<Vec<Vec<f32>>>,
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

        Ok(Collection { pcks, raw: None })
    }

    pub fn from_file_raw(p: impl AsRef<Path>) -> anyhow::Result<Collection> {
        let p = p.as_ref();
        let mut b = std::fs::read(p)?;

        if (b.len() % RAW_PACKAGE_SZ) != 0 {
            eprintln!("Warning, collection consists of non-integer number of packages.");
        }

        let n = b.len() / RAW_PACKAGE_SZ;

        eprintln!(
            "Parsing {} bytes of packages into {} packages..",
            b.len(),
            n
        );
        let (pcks, raw) = b
            .chunks_exact_mut(RAW_PACKAGE_SZ)
            .filter_map(|p| {
                let (p, raw) = p.split_at_mut(axl::AXL_POSTCARD_SZ);

                let raw = VecRawAxl::from_slice(bytemuck::cast_slice(raw)).unwrap();
                let raw = raw.iter().map(|v| (*v).into()).collect::<Vec<f32>>();

                match postcard::from_bytes_cobs(p) {
                    Ok(p) => Some((p, raw)),
                    Err(e) => {
                        eprintln!("failed to parse package: {:?}", e);
                        None
                    }
                }
            })
            .unzip();

        Ok(Collection {
            pcks,
            raw: Some(raw),
        })
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

    #[test]
    fn open_raw() {
        let c = Collection::from_file_raw("tests/data/14.3").unwrap();
        println!("packages: {}", c.pcks.len());

        // println!("raw: {:?}", c.raw.unwrap()[0]);

        let last = &c.raw.unwrap()[0];
        println!("{}", json::to_string(&last).unwrap());

        // doesn't work due to: https://github.com/starkat99/half-rs/issues/60
        let f = half::f16::from_f32(45.0);
        println!("{}", json::to_string(&f).unwrap());
    }

    #[test]
    fn open_regular_v6() {
        let c = Collection::from_file("tests/data/3.6").unwrap();
        println!("packages: {}", c.pcks.len());

        assert!(c.raw.is_none());
        assert_eq!(c.pcks.len(), 4);
    }

    #[ignore]
    #[test]
    fn open_raw_v5() {
        let c = Collection::from_file_raw("tests/data/32.5").unwrap();
        println!("packages: {}", c.pcks.len());

        assert!(c.raw.is_some());
        assert_eq!(c.pcks.len(), 32);

        // println!("raw: {:?}", c.raw.unwrap()[0]);

        let last = &c.raw.unwrap()[0];
        println!("{}", json::to_string(&last).unwrap());
    }
}
