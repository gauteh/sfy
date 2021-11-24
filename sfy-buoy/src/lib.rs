#![cfg_attr(not(test), no_std)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

use ambiq_hal as hal;

pub mod note;

