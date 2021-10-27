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
/// The Shabby Data host.
struct Sby {
    /// configuration file.
    #[argh(option, short = 'c', default = "PathBuf::from(\"sby-hub.toml\")")]
    config: PathBuf,
}

mod buoys;
mod config;
mod database;

pub type State = Arc<tokio::sync::RwLock<database::Database>>;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("warn,sby_data=debug")).init();

    info!("sby-data server");
    let sby: Sby = argh::from_env();
    let config = config::Config::from_path(sby.config);

    let database = config.database.expect("no database path specified");
    let database = database::Database::open(database)?;
    let database = Arc::new(tokio::sync::RwLock::new(database));

    // build our application with a single route
    // let api = warp::path!("").map(|| "hello!");
    let api = buoys::filters(database);

    info!("listening on: {:?}", config.address);
    warp::serve(api).run(config.address).await;

    Ok(())
}
