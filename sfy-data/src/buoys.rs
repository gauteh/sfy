//! End-points for buoys.

use crate::State;
use std::convert::Infallible;
use std::sync::Arc;
use futures_util::future;
use warp::{reject, Filter, Rejection};

pub fn filters(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoy" / String)
        .and(check_token(state.clone()))
        .and(warp::body::content_length_limit(50 * 1024 * 1024))
        .and(warp::body::bytes())
        .and(with_state(state.clone()))
        .and_then(append)
}

fn with_state(state: State) -> impl Filter<Extract = (State,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&state))
}

fn check_token(state: State) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::<String>("SFY_AUTH_TOKEN").and_then(move |v: String| {
        if state.config.tokens.contains(&v) {
            future::ok(())
        } else {
            warn!("rejected token: {}", v);
            future::err(reject::not_found())
        }
    }).untuple_one()
}

async fn append(
    buoy: String,
    body: bytes::Bytes,
    state: State,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !state.config.buoys.iter().any(|b| b.id == buoy) {
        return Err(warp::reject::not_found());
    }
    info!("got message for buoy: {}", buoy);

    Ok(warp::reply())
}
