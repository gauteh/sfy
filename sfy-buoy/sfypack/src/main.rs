use std::path::PathBuf;
use argh::FromArgs;

use sfypack;

#[derive(FromArgs)]
/// Load and print Axl package from binary collection.
struct SfyPack {
    #[argh(positional, description = "file name")]
    file: PathBuf
}

fn main() -> anyhow::Result<()> {
    let pck: SfyPack = argh::from_env();
    println!("Loading collection from: {:?}", pck.file);

    Ok(())
}
