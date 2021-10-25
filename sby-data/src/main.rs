#[macro_use]
extern crate log;
use argh::FromArgs;
use axum::{handler::get, Router};
use env_logger::Env;
use std::path::PathBuf;
#[macro_use]
extern crate eyre;

#[derive(FromArgs)]
/// The Shabby Data host.
struct Sby {
    /// configuration file.
    #[argh(option, short = 'c')]
    config: Option<PathBuf>,
}

mod config;
mod database;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("warn,sby_data=debug")).init();

    info!("sby-data server");
    let sby: Sby = argh::from_env();
    let config = sby
        .config
        .map_or_else(|| config::Config::default(), config::Config::from_path);

    let database = database::Database::open(config.database.expect("no database path specificed"))?;

    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    info!("listening on: {:?}", config.address);
    // run it with hyper on localhost:3000
    axum::Server::bind(&config.address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
