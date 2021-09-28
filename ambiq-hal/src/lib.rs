#![no_std]

pub extern crate embedded_hal as hal;
pub extern crate ambiq_apollo3_pac as pac;

pub mod clock;
pub mod time;
pub mod delay;

pub use hal::prelude;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
