#![cfg_attr(not(test), no_std)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

pub mod note;
pub mod waves;

