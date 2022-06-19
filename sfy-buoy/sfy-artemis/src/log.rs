use sfy::note::Notecarrier;
use ambiq_hal::i2c;

/// A reference to the Notecarrier once it is initialized. The idea is that
/// it can be used from reset routines to transfer log messages. In that case the main thread will
/// not be running anyway.
pub static mut NOTE: Option<*mut Notecarrier<i2c::Iom4>> = None;

