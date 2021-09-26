use serde::Deserialize;

#[derive(Deserialize)]
pub struct Status {
    status: heapless::String<10>,
    usb: bool,
    storage: usize,
    time: u64,
    connected: bool
}

pub fn status() -> Result<Status, ()> {
    serde_json_core::from_str(
    r#"{
    "status":    "{normal}",
    "usb":       true,
    "storage":   8,
    "time":      1599684765,
    "connected": "true"
    }"#).map_err(|_| ()).map(|(a,_)| a)
}

#[cfg(test)]
mod tests {

}
