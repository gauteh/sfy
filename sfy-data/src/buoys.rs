//! End-points for buoys.

use crate::State;
use std::sync::Arc;
use warp::Filter;

pub fn filters(
    db: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone{
    let db = db.clone();

    warp::path!("buoy" / String)
        .and(warp::body::content_length_limit(50 * 1024 * 1024))
        .and(warp::body::bytes())
        .and_then(move |s, b| {
            let db = db.clone();
            async move { append(Arc::clone(&db), s, b).await }
        })
}

async fn append(
    db: State,
    buoy: String,
    body: bytes::Bytes,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Err(warp::reject::not_found())
    Ok(warp::reply())
}
