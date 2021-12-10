//! End-points for buoys.

use crate::State;
use futures_util::future;
use serde_json as json;
use std::convert::Infallible;
use std::sync::Arc;
use warp::{http::StatusCode, reject, Filter, Rejection, Reply};
use sanitize_filename::sanitize;

pub fn filters(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoy")
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
    warp::header::<String>("SFY_AUTH_TOKEN")
        .and_then(move |v: String| {
            if state.config.tokens.contains(&v) {
                future::ok(())
            } else {
                warn!("rejected token: {}", v);
                future::err(reject::not_found())
            }
        })
        .untuple_one()
}

struct Event {
    event: String,
    device: String,
    file: Option<String>,
    body: json::Value,
}

fn parse_data(body: &[u8]) -> eyre::Result<Event> {
    let body: json::Value = json::from_slice(&body)?;

    let event = body
        .get("event")
        .and_then(json::Value::as_str)
        .map(String::from)
        .ok_or(eyre!("no event field"))?;

    let device = body
        .get("device")
        .and_then(json::Value::as_str)
        .map(String::from)
        .ok_or(eyre!("no dev field"))?;

    let file = body
        .get("file")
        .and_then(json::Value::as_str)
        .map(String::from);

    Ok(Event {
        event,
        device,
        file,
        body,
    })
}

// async fn handle_reject(err: Rejection) -> Result<impl Reply, Infallible> {}

#[derive(Debug)]
pub enum AppendErrors {
    Database,
}

impl reject::Reject for AppendErrors {}

async fn append(body: bytes::Bytes, state: State) -> Result<impl warp::Reply, warp::Rejection> {
    debug!("got message: {:#?}", body);

    if let Ok(event) = parse_data(&body) {
        let device = sanitize(&event.device);

        info!(
            "event: {} from {}({}) to file: {:?}",
            event.event, event.device, device, event.file
        );

        let mut db = state.db.write().await;
        let mut b = db
            .buoy(&device)
            .map_err(|e| {
                error!("failed to open database for device: {}: {:?}", &device, e);
                reject::custom(AppendErrors::Database)
            })?;

        let file = &format!("{}_{}.json", event.event, event.file.unwrap_or("__unnamed__".into()));
        let file = sanitize(&file);
        debug!("writing to: {}", file);

        b.append(&file, &body).await.map_err(|e| {
            error!("failed to write file: {:?}", e);
            reject::custom(AppendErrors::Database)
        })?;

        Ok("".into_response())
    } else {
        warn!("could not parse event, storing event in lost+found");
        Ok(StatusCode::BAD_REQUEST.into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_sensor_db() {
        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();
        let parsed = parse_data(&event).unwrap();

        assert_eq!(parsed.event, "9ef2e080-f0b4-4036-8ccc-ec4206553537");
        assert_eq!(parsed.device, "dev:864475044203262");
        assert_eq!(parsed.file, Some(String::from("sensor.db")));
    }

    #[tokio::test]
    async fn check_token_ok() {
        let (_, state) = crate::test_state();

        let f = check_token(state);

        assert!(warp::test::request()
            .header("SFY_AUTH_TOKEN", "wrong-token")
            .filter(&f)
            .await
            .is_err());

        assert!(warp::test::request()
            .header("SFY_AUTH_TOKEN", "token1")
            .filter(&f)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn append_to_database() {
        let (tmp, state) = crate::test_state();
        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();

        let f = filters(state);

        let res = warp::test::request()
            .path("/buoy")
            .header("SFY_AUTH_TOKEN", "wrong-token")
            .body(&event)
            .reply(&f)
            .await;

        assert!(res.status() != 200);

        let res = warp::test::request()
            .path("/buoy")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let path = tmp.path().join("dev864475044203262").join("9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json");
        assert_eq!(
            std::fs::read(path).unwrap().as_slice(),
            &event
        );
    }

    #[tokio::test]
    async fn bad_event() {
        let (_, state) = crate::test_state();
        let event = br#"{ "noevent": "asdf", "something": "hey", bad json even }"#;

        let f = filters(state);

        let res = warp::test::request()
            .path("/buoy")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 400);
    }
}
