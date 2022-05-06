//! End-points for buoys.

use crate::State;
use futures_util::future;
use sanitize_filename::sanitize;
use serde_json as json;
use std::convert::Infallible;
use std::sync::Arc;
use warp::{http::Response, http::StatusCode, reject, Filter, Rejection, Reply};

pub fn filters(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    append(state.clone())
        .or(list(state.clone()))
        .or(entries(state.clone()))
        .or(last(state.clone()))
        .or(entry(state.clone()))
}

pub fn append(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoy")
        .and(warp::post())
        .and(check_token(state.clone()))
        .and(warp::body::content_length_limit(50 * 1024 * 1024))
        .and(warp::body::bytes())
        .and(with_state(state.clone()))
        .and_then(handlers::append)
}

pub fn list(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys")
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::list)
}

pub fn entries(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys" / String)
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::entries)
}

pub fn entry(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys" / String / String)
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::entry)
}

pub fn last(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys" / String / "last")
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::last)
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

fn check_read_token(state: State) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::<String>("SFY_AUTH_TOKEN")
        .and_then(move |v: String| {
            if state.config.read_tokens.contains(&v) {
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
    name: Option<String>,
    file: Option<String>,
    #[allow(unused)]
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

    let name = body
        .get("sn")
        .and_then(json::Value::as_str)
        .map(String::from);

    let file = body
        .get("file")
        .and_then(json::Value::as_str)
        .map(String::from);

    Ok(Event {
        event,
        device,
        name,
        file,
        body,
    })
}

// async fn handle_reject(err: Rejection) -> Result<impl Reply, Infallible> {}

#[derive(Debug)]
pub enum AppendErrors {
    Database,
    Internal,
}

impl reject::Reject for AppendErrors {}

impl Into<AppendErrors> for eyre::ErrReport {
    fn into(self: eyre::ErrReport) -> AppendErrors {
        AppendErrors::Internal
    }
}

pub mod handlers {
    use super::*;

    pub async fn list(state: State) -> Result<impl warp::Reply, warp::Rejection> {
        let buoys = state
            .db
            .read()
            .await
            .buoys()
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;
        Ok(warp::reply::json(&buoys))
    }

    pub async fn entries(buoy: String, state: State) -> Result<impl warp::Reply, warp::Rejection> {
        let buoy = sanitize(buoy);

        let entries = state
            .db
            .write()
            .await
            .buoy(&buoy).await
            .map_err(|_| reject::custom(AppendErrors::Internal))?
            .entries()
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;
        Ok(warp::reply::json(&entries))
    }

    pub async fn entry(
        buoy: String,
        entry: String,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let buoy = sanitize(buoy);
        let entry = sanitize(entry);

        let entry = state
            .db
            .write()
            .await
            .buoy(&buoy)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?
            .get(entry)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;

        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(entry))
    }

    pub async fn last(
        buoy: String,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let buoy = sanitize(buoy);

        let entry = state
            .db
            .write()
            .await
            .buoy(&buoy)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?
            .last()
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;

        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(entry))
    }

    pub async fn append(
        body: bytes::Bytes,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        trace!("got message: {:#?}", body);

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| reject::custom(AppendErrors::Internal))?;

        let now = if cfg!(test) { 0 } else { now.as_millis() };

        if let Ok(event) = parse_data(&body) {
            let device = sanitize(&event.device);

            info!(
                "event: {} from {}({}) to file: {:?}",
                event.event, event.device, device, event.file
            );

            let db = state.db.write().await;
            let mut b = db.buoy(&device).await.map_err(|e| {
                error!("failed to open database for device: {}: {:?}", &device, e);
                reject::custom(AppendErrors::Database)
            })?;

            let file = &format!(
                "{}-{}_{}.json",
                now,
                event.event,
                event.file.unwrap_or("__unnamed__".into())
            );
            let file = sanitize(&file);
            debug!("writing to: {}", file);

            b.append(event.name, &file, &body).await.map_err(|e| {
                error!("failed to write file: {:?}", e);
                reject::custom(AppendErrors::Database)
            })?;

            Ok("".into_response())
        } else {
            warn!("could not parse event, storing event in lost+found");

            let db = state.db.write().await;
            let mut b = db.buoy("lost+found").await.map_err(|e| {
                error!("failed to open database for lost+found: {:?}", e);
                reject::custom(AppendErrors::Database)
            })?;

            let file = &format!("{}.json", now,);
            let file = sanitize(&file);
            debug!("writing to: {}", file);

            b.append(None, &file, &body).await.map_err(|e| {
                error!("failed to write file: {:?}", e);
                reject::custom(AppendErrors::Database)
            })?;

            Ok(StatusCode::BAD_REQUEST.into_response())
        }
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
        let state = crate::test_state().await;

        let f = check_token(state);

        assert!(warp::test::request()
            .method("POST")
            .header("SFY_AUTH_TOKEN", "wrong-token")
            .filter(&f)
            .await
            .is_err());

        assert!(warp::test::request()
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .filter(&f)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn append_to_database() {
        let state = crate::test_state().await;
        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();

        let f = filters(state.clone());

        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "wrong-token")
            .body(&event)
            .reply(&f)
            .await;

        assert!(res.status() != 200);

        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let e = state.db.read().await.buoy("dev864475044203262").await.unwrap().get("0-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json").await.unwrap();

        assert_eq!(&e, &event);
    }

    #[tokio::test]
    async fn bad_event() {
        let state = crate::test_state().await;
        let event = br#"{ "noevent": "asdf", "something": "hey", bad json even }"#;

        let f = filters(state);

        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 400);
    }

    #[tokio::test]
    async fn list_buoys() {
        let state = crate::test_state().await;
        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();

        let f = filters(state);

        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        // Missing token
        let res = warp::test::request()
            .path("/buoys")
            .method("GET")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 400);

        let res = warp::test::request()
            .path("/buoys")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);
        assert_eq!(res.body(), "[[\"dev864475044203262\",\"cain\"]]");
    }

    #[tokio::test]
    async fn list_entries() {
        let state = crate::test_state().await;
        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();

        let f = filters(state);

        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        // Missing token
        let res = warp::test::request()
            .path("/buoys/dev864475044203262")
            .method("GET")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 400);

        let res = warp::test::request()
            .path("/buoys/dev864475044203262")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.body(),
            "[\"0-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json\"]"
        );
    }

    #[tokio::test]
    async fn get_entry() {
        let state = crate::test_state().await;
        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();

        let f = filters(state);

        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        // Missing token
        let res = warp::test::request()
            .path("/buoys/dev864475044203262/0-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json")
            .method("GET")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 400);

        let res = warp::test::request()
            .path("/buoys/dev864475044203262/0-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );
        assert_eq!(res.body(), &event);
    }

    #[tokio::test]
    async fn last_entry() {
        let state = crate::test_state().await;

        let f = filters(state);

        let event = std::fs::read("tests/events/sensor.db_01.json").unwrap();
        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let res = warp::test::request()
            .path("/buoys/dev864475044203262/last")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 500);

        let event = std::fs::read("tests/events/1647870799330-1876870b-4708-4366-8db5-68f872cc4e6d_axl.qo.json").unwrap();
        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let res = warp::test::request()
            .path("/buoys/dev867730051260788/last")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );
        assert_eq!(res.body(), &event);
    }
}
