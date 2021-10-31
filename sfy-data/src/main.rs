#[macro_use]
extern crate log;

use argh::FromArgs;
use env_logger::Env;
use std::path::PathBuf;
use std::sync::Arc;
use warp::Filter;

#[macro_use]
extern crate eyre;

#[derive(FromArgs)]
/// The Small Friendly Data host.
struct Sfy {
    /// configuration file.
    #[argh(option, short = 'c', default = "PathBuf::from(\"sfy-data.toml\")")]
    config: PathBuf,
}

mod buoys;
mod config;
mod database;

pub type State = Arc<tokio::sync::RwLock<database::Database>>;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::Builder::from_env(
        Env::default().default_filter_or("warn,sfy_data=debug,warp=debug"),
    )
    .init();

    info!("sfy-data server");
    let sfy: Sfy = argh::from_env();
    let config = config::Config::from_path(sfy.config);

    let database = config.database.expect("no database path specified");
    let database = database::Database::open(database)?;
    let database = Arc::new(tokio::sync::RwLock::new(database));

    let api = buoys::filters(database).with(warp::log("sfy_data::api"));

    info!("listening on: {:?}", config.address);
    warp::serve(api).run(config.address).await;

    Ok(())
}
