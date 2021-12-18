#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use panic_probe as _; // TODO: Restart board on panic.

#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};

#[cfg(not(test))]
use cortex_m_rt::entry;

use ambiq_hal::{self as hal, prelude::*};
use defmt_rtt as _;
use hal::i2c;
use chrono::NaiveDate;

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

    let i2c = i2c::I2c::new(dp.IOM2, pins.d17, pins.d18, i2c::Freq::F100kHz);
    let bus = shared_bus::BusManagerSimple::new(i2c);

    // Set up RTC
    let mut rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
    rtc.set(NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0)); // Now timestamps will be positive.
    rtc.enable();

    println!("hello from sfy!");

    info!("Setting up Notecarrier..");
    let mut note = Notecarrier::new(bus.acquire_i2c(), &mut delay).unwrap();

    info!("Setting up IMU..");
    let mut waves = Waves::new(bus.acquire_i2c()).unwrap();
    waves.take_buf(rtc.now().timestamp_millis() as u32).unwrap(); // set timestamp.

    // Subsystem state
    let mut location = sfy::Location::new();
    let mut imu = sfy::Imu::default();

    info!("Enable IMU.");
    waves.enable_fifo(&mut delay).unwrap();

    info!("Entering main loop");

    loop {
        led.toggle().unwrap();

        location.check_retrieve(&mut rtc, &mut delay, &mut note).unwrap();
        imu.check_retrieve(&mut rtc, &mut waves).unwrap();


        // Check if IMU queue is full
        if imu.dequeue.is_full() { // or IN_DRAINING_QUEUE
        }
        // Take and queue package for notecard, but only one for each iteration untill the
        // queue is empty.
        //
    }
}
