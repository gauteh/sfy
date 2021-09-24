#[macro_use]
extern crate log;
use env_logger::Env;

use axum::{
    handler::get,
    Router,
};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn,sby_data=debug")).init();

    info!("Hello, world!");

    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
