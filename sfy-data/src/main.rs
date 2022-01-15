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

pub struct SfyState {
    pub db: tokio::sync::RwLock<database::Database>,
    pub config: config::Config,
}

pub type State = Arc<SfyState>;

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

    let database = config.database.clone().expect("no database path specified");
    let database = database::Database::open(database)?;
    let database = tokio::sync::RwLock::new(database);

    let state = Arc::new(SfyState {
        db: database,
        config: config.clone(),
    });

    info!("listening on: {:?}", config.address);

    let cors = warp::cors().allow_any_origin().allow_header("SFY_AUTH_TOKEN");


    if let Some(dir) = config.files {
        info!("serving files in directory: {:?}", dir);
        let api = warp::path("sfy").and(warp::fs::dir(dir));
        let api = api.or(buoys::filters(state)).with(cors).with(warp::log("sfy_data::api"));
        warp::serve(api).run(config.address).await;
    } else {
        let api = buoys::filters(state).with(cors).with(warp::log("sfy_data::api"));
        warp::serve(api).run(config.address).await;
    };


    Ok(())
}

#[cfg(test)]
fn test_state() -> (tempfile::TempDir, State) {
    let config = config::Config::test_config();
    let (dir, db) = database::Database::temporary();
    let db = tokio::sync::RwLock::new(db);

    let state = SfyState { config, db };
    let state = Arc::new(state);

    (dir, state)
}
