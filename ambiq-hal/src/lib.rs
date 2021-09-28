#![no_std]

pub use embedded_hal as hal;

// pub mod delay;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
