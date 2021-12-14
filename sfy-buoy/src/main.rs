#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use panic_probe as _; // TODO: Restart board on panic.

#[allow(unused_imports)]
use defmt::{println, debug, error, info, trace, warn};

#[cfg(not(test))]
use cortex_m_rt::entry;

use defmt_rtt as _;
use ambiq_hal::{self as hal, prelude::*};

use hal::i2c;

use sfy::note::Notecarrier;
use sfy::waves::Waves;

#[cfg_attr(not(test), entry)]
fn main() -> ! {
    unsafe {
        // Set the clock frequency.
        halc::am_hal_clkgen_control(
            halc::am_hal_clkgen_control_e_AM_HAL_CLKGEN_CONTROL_SYSCLK_MAX,
            0 as *mut c_void,
        );

        // Set the default cache configuration
        halc::am_hal_cachectrl_config(&halc::am_hal_cachectrl_defaults);
        halc::am_hal_cachectrl_enable();

        // Configure the board for low power operation.
        halc::am_bsp_low_power_init();
    }

    let mut dp = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();
    let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

    let pins = hal::gpio::Pins::new(dp.GPIO);
    let mut led = pins.d19.into_push_pull_output(); // d14 on redboard_artemis

    let i2c = i2c::I2c::new(dp.IOM2, pins.d17, pins.d18, i2c::Freq::F400kHz);
    let bus = shared_bus::BusManagerSimple::new(i2c);

    println!("hello from sfy!");

    // info!("Setting up Notecarrier..");
    // let mut note = Notecarrier::new(bus.acquire_i2c());

    info!("Setting up IMU..");
    let mut waves = Waves::new(bus.acquire_i2c()).unwrap();

    info!("Entering main loop");

    loop {
        delay.delay_ms(2000u32);
        led.toggle().unwrap();

        let temp = waves.get_temperature().unwrap();
        info!("Temperature: {}", temp);

        waves.iter();
        // Subsystems:
        // - waves
        // - cellular (note)
    }
}

