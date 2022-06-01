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
        .or(append_omb(state.clone()))
        .or(list(state.clone()))
        .or(entries(state.clone()))
        .or(last(state.clone()))
        .or(storage_info(state.clone()))
        .or(range(state.clone()))
        .or(list_range(state.clone()))
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

pub fn append_omb(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoy" / "omb")
        .and(warp::post())
        .and(check_token(state.clone()))
        .and(warp::body::content_length_limit(50 * 1024 * 1024))
        .and(warp::body::bytes())
        .and(with_state(state.clone()))
        .and_then(handlers::append_omb)
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

pub fn storage_info(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys" / String / "storage_info")
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::storage_info)
}

pub fn range(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys" / String / "from" / i64 / "to" / i64)
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::range)
}

pub fn list_range(
    state: State,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let state = state.clone();

    warp::path!("buoys" / "list" / String / "from" / i64 / "to" / i64)
        .and(warp::get())
        .and(check_read_token(state.clone()))
        .and(with_state(state.clone()))
        .and_then(handlers::list_range)
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

#[derive(Debug)]
struct OmbEvent {
    device: String,
    account: String,
    received: u64,
    message_type: crate::database::OmbMessageType,
    #[allow(unused)]
    body: json::Value,
}

fn parse_omb_data(body: &[u8]) -> eyre::Result<OmbEvent> {
    let body: json::Value = json::from_slice(&body)?;

    let device = body
        .get("device")
        .and_then(json::Value::as_str)
        .map(String::from)
        .ok_or(eyre!("no dev field"))?;

    let account = body
        .get("account")
        .and_then(json::Value::as_str)
        .map(String::from)
        .ok_or(eyre!("no account field"))?;


    let received = body
        .get("datetime")
        .and_then(json::Value::as_f64)
        .ok_or(eyre!("no datetime field"))?;
    let received = received.trunc() as u64;

    let message_type = body.get("type")
        .and_then(json::Value::as_str)
        .ok_or(eyre!("no type field"))?
        .into();


    Ok(OmbEvent {
        device,
        received,
        message_type,
        account,
        body,
    })
}

struct Event {
    event: String,
    device: String,
    received: u64,
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

    let received = body
        .get("received")
        .and_then(json::Value::as_f64)
        .unwrap_or(0.0);
    let received = (received * 1000.).trunc() as u64;

    let file = body
        .get("file")
        .and_then(json::Value::as_str)
        .map(String::from);

    Ok(Event {
        event,
        device,
        received,
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
            .buoy(&buoy)
            .await
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

    pub async fn last(buoy: String, state: State) -> Result<impl warp::Reply, warp::Rejection> {
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

    pub async fn storage_info(
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
            .storage_info()
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;

        Ok(warp::reply::json(&entry))
    }

    pub async fn range(
        buoy: String,
        from: i64,
        to: i64,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let buoy = sanitize(buoy);

        let entries = state
            .db
            .write()
            .await
            .buoy(&buoy)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?
            .get_range(from, to)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;

        Ok(warp::reply::json(&entries))
    }

    pub async fn list_range(
        buoy: String,
        from: i64,
        to: i64,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let buoy = sanitize(buoy);

        let entries = state
            .db
            .write()
            .await
            .buoy(&buoy)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?
            .list_range(from, to)
            .await
            .map_err(|_| reject::custom(AppendErrors::Internal))?;

        let entries: Vec<String> = entries
            .into_iter()
            .map(|e| format!("{}-{}", e.0, e.1))
            .collect();

        Ok(warp::reply::json(&entries))
    }

    pub async fn append(
        body: bytes::Bytes,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        trace!("got message: {:#?}", body);

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
                "{}_{}.json",
                event.event,
                event.file.unwrap_or("__unnamed__".into())
            );
            let file = sanitize(&file);
            debug!("writing to: {}", file);

            b.append(event.name, &file, event.received, &body)
                .await
                .map_err(|e| {
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

            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| reject::custom(AppendErrors::Internal))?;

            let now = if cfg!(test) { 0 } else { now.as_millis() };

            let file = &format!("{}.json", now,);
            let file = sanitize(&file);
            debug!("writing to: {}", file);

            b.append(None, &file, now as u64, &body)
                .await
                .map_err(|e| {
                    error!("failed to write file: {:?}", e);
                    reject::custom(AppendErrors::Database)
                })?;

            Ok(StatusCode::BAD_REQUEST.into_response())
        }
    }

    pub async fn append_omb(
        body: bytes::Bytes,
        state: State,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        trace!("got message: {:#?}", body);

        if let Ok(event) = parse_omb_data(&body) {
            let device = sanitize(&event.device);

            info!(
                "omb event: {:?}",
                event
            );

            let db = state.db.write().await;
            let mut b = db.buoy(&device).await.map_err(|e| {
                error!("failed to open database for device: {}: {:?}", &device, e);
                reject::custom(AppendErrors::Database)
            })?;


            b.append_omb(event.account, event.received, event.message_type, &body)
                .await
                .map_err(|e| {
                    error!("failed to write file: {:?}", e);
                    reject::custom(AppendErrors::Database)
                })?;

            return Ok("".into_response());
        }

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

    #[test]
    fn test_parse_omb_event() {
        let event = std::fs::read("tests/events/01-omb.json").unwrap();
        let parsed = parse_omb_data(&event).unwrap();
        println!("{:?}", parsed);

        let event = br##"{"account": "gauteh@met.no", "datetime": 1654003378000.0, "device": "NOFO-OPV-2022-01", "type": "gps", "body": {"iridium_pos": {"lat": 58.92556666666667, "lon": 5.987166666666667}, "messages": [{"datetime_fix": 1654003274.0, "latitude": 58.8867932, "longitude": 5.7136341, "is_valid": true}]}}"##;
        let parsed = parse_omb_data(event).unwrap();
        println!("{:?}", parsed);
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

        let e = state
            .db
            .read()
            .await
            .buoy("dev864475044203262")
            .await
            .unwrap()
            .get("1639059643089-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json")
            .await
            .unwrap();

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
        assert_eq!(res.body(), "[[\"dev864475044203262\",\"cain\",\"sfy\"]]");
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
            "[\"1639059643089-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json\"]"
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
            .path("/buoys/dev864475044203262/1639059643089-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json")
            .method("GET")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 400);

        let res = warp::test::request()
            .path("/buoys/dev864475044203262/1639059643089-9ef2e080-f0b4-4036-8ccc-ec4206553537_sensor.db.json")
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

        let event = std::fs::read(
            "tests/events/1647870799330-1876870b-4708-4366-8db5-68f872cc4e6d_axl.qo.json",
        )
        .unwrap();
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

    #[tokio::test]
    async fn range() {
        use crate::database::Event;

        let state = crate::test_state().await;

        let f = filters(state);

        let events = [
            r#"{"received": 0, "event": "event-0", "device": "0", "name": "0", "file": "axl.qo" }"#,
            r#"{"received": 1, "event": "event-1", "device": "0", "name": "0", "file": "axl.qo" }"#,
            r#"{"received": 2, "event": "event-2", "device": "0", "name": "0", "file": "axl.qo" }"#,
            r#"{"received": 3, "event": "event-3", "device": "0", "name": "0", "file": "axl.qo" }"#,
        ];

        for e0 in events {
            println!("posting: {}", e0);
            let res = warp::test::request()
                .path("/buoy")
                .method("POST")
                .header("SFY_AUTH_TOKEN", "token1")
                .body(&e0)
                .reply(&f)
                .await;

            assert_eq!(res.status(), 200);
        }

        // Received is multiplied to 1000 in parse_data.

        let res = warp::test::request()
            .path("/buoys/0/from/0/to/2000")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let revents: Vec<Event> = json::from_slice(res.body()).unwrap();
        println!("{:?}", revents);

        assert_eq!(revents.len(), 3);

        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );

        assert_eq!(revents[2].received, 2000);

        let res = warp::test::request()
            .path("/buoys/0/from/2000/to/3000")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let revents: Vec<Event> = json::from_slice(res.body()).unwrap();
        println!("{:?}", revents);

        assert_eq!(revents.len(), 2);

        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );

        assert_eq!(revents[1].received, 3000);

        // List
        let res = warp::test::request()
            .path("/buoys/list/0/from/2000/to/3000")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let revents: Vec<String> = json::from_slice(res.body()).unwrap();

        assert_eq!(revents.len(), 2);

        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );

        println!("{revents:?}");
        assert_eq!(
            &revents,
            &["2000-event-2_axl.qo.json", "3000-event-3_axl.qo.json"]
        );
    }

    #[tokio::test]
    async fn storage_info() {
        let state = crate::test_state().await;

        let f = filters(state);

        let event = std::fs::read(
            "tests/events/1653994017660-ae50c1e9-0800-4fd9-9cb6-cdd6a6d08eb3_storage.db.json",
        )
        .unwrap();
        let res = warp::test::request()
            .path("/buoy")
            .method("POST")
            .header("SFY_AUTH_TOKEN", "token1")
            .body(&event)
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        let res = warp::test::request()
            .path("/buoys/dev864475044204278/storage_info")
            .method("GET")
            .header("SFY_AUTH_TOKEN", "r-token1")
            .reply(&f)
            .await;

        assert_eq!(res.status(), 200);

        assert_eq!(
            res.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );

        let info: crate::database::StorageInfo = json::from_slice(res.body()).unwrap();
        assert_eq!(info.current_id, Some(40002));
        assert_eq!(info.request_start, None);
        assert_eq!(info.request_end, None);
    }
}
