//! End-points for buoys.

use crate::State;
use futures_util::future;
use serde_json as json;
use std::convert::Infallible;
use std::sync::Arc;
use warp::{reject, Filter, Rejection};

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

async fn append(body: bytes::Bytes, state: State) -> Result<impl warp::Reply, warp::Rejection> {
    debug!("got message: {:#?}", body);

    if let Ok(event) = parse_data(&body) {
    } else {
        warn!("could not parse event, storing event in lost+found");
    }

    Ok(warp::reply())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_sensor_db() {
        let event = br#"{
    "event": "9ef2e080-f0b4-4036-8ccc-ec4206553537",
    "session": "8cceb49b-5ddf-46e4-80cb-d527959511e2",
    "best_id": "cain",
    "device": "dev:864475044203262",
    "sn": "cain",
    "product": "product:no.met.gauteh:sfy",
    "received": 1639059643.08987,
    "routed": 1639059646,
    "req": "note.add",
    "when": 1639059596,
    "file": "sensor.db",
    "note": "WQ1620",
    "body": {
        "t0": 300,
        "v": [
            7.0,
            6.0,
            4.0
        ]
    },
    "best_location_type": "tower",
    "best_lat": 60.3302875,
    "best_lon": 5.371703125,
    "best_location": "Sandsli",
    "best_country": "NO",
    "best_timezone": "Europe/Oslo",
    "tower_when": 1639059642,
    "tower_lat": 60.3302875,
    "tower_lon": 5.371703125,
    "tower_country": "NO",
    "tower_location": "Sandsli",
    "tower_timezone": "Europe/Oslo",
    "tower_id": "242,1,11001,12313",
    "logattn": true,
    "log": {
        "app:4c5e935c-7acb-4f20-bca0-cda95e9fd1d2/route:7a94fb193c625c7d3b118ade5edc3b2e": {
            "attn": true,
            "status": "404"
        }
    }
}"#;
        let parsed = parse_data(event).unwrap();

        assert_eq!(parsed.event, "9ef2e080-f0b4-4036-8ccc-ec4206553537");
        assert_eq!(parsed.device, "dev:864475044203262");
        assert_eq!(parsed.file, Some(String::from("sensor.db")));
    }
}
