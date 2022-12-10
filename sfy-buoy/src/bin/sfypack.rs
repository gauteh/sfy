use argh::FromArgs;
use chrono::NaiveDateTime;
use serde_json as json;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use sfy::axl;
use sfy::storage::PACKAGE_SZ;
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
}

fn main() -> anyhow::Result<()> {
    let pck: SfyPack = argh::from_env();
    eprintln!("Loading collection from: {:?}", pck.file);

    let c = match pck.file.extension().map(|s| s.to_str()).flatten() {
        Some("1") => Collection::from_file(&pck.file),
        Some("3") => Collection::from_file_v3(&pck.file),
        _ => Err(anyhow::anyhow!("Unknown extension."))
    }?;
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
            println!("{}", json::to_string_pretty(&c).unwrap());
            // if let Some(raw) = c.raw {
            //     println!("{}", json::to_string_pretty(&raw).unwrap());
            // }
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
        let (body, b64) = pck.split();

        let payload = String::from_utf8(b64.as_slice().to_vec()).unwrap();

        AxlNote { body, payload }
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

    pub fn from_file_v3(p: impl AsRef<Path>) -> anyhow::Result<Collection> {
        let p = p.as_ref();
        let mut b = std::fs::read(p)?;

        if (b.len() % PACKAGE_SZ) != 0 {
            eprintln!("Warning, collection consists of non-integer number of packages.");
        }

        let n = b.len() / PACKAGE_SZ;

        eprintln!(
            "Parsing {} bytes of packages into {} packages..",
            b.len(),
            n
        );
        let (pcks, raw) = b
            .chunks_exact_mut(PACKAGE_SZ)
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

        Ok(Collection { pcks, raw: Some(raw) })
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
    fn open_v3() {
        let c = Collection::from_file_v3("tests/data/14.3").unwrap();
        println!("packages: {}", c.pcks.len());

        // println!("raw: {:?}", c.raw.unwrap()[0]);

        let last = &c.raw.unwrap()[0];
        println!("{}", json::to_string(&last).unwrap());

        // doesn't work due to: https://github.com/starkat99/half-rs/issues/60
        let f = half::f16::from_f32(45.0);
        println!("{}", json::to_string(&f).unwrap());
    }
}
