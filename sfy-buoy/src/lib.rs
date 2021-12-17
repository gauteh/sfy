#![cfg_attr(not(test), no_std)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

pub mod note;
pub mod waves;

#[derive(Default, Debug, defmt::Format)]
pub struct Location {
    pub lat: f32,
    pub lon: f32,
    pub time: u32,

    pub retrived: Option<u32>,
    pub last_tried: Option<u32>,
}


#[derive(Default)]
pub struct Imu {
    pub dequeue: heapless::Deque<note::AxlPacket, 60>,
    pub last_poll: u32,
}
