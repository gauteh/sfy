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
    waves.take_buf(rtc.now().timestamp_millis() as u32, 0.0, 0.0).unwrap(); // set timestamp.

    // Subsystem state
    let mut location = sfy::Location::new();
    let mut imu = sfy::Imu::new();

    info!("Enable IMU.");
    waves.enable_fifo(&mut delay).unwrap();

    info!("Entering main loop");

    loop {
        led.toggle().unwrap();

        // Retrieve and set time and location.
        location.check_retrieve(&mut rtc, &mut delay, &mut note).unwrap();

        // This is the most critical part. If it turns out that the other parts sometimes
        // take too long time we need to move this to either an RTC interrupt or DRDY
        // interrupt.
        imu.check_retrieve(&mut rtc, &mut waves, &location).unwrap();

        // XXX: Draining the queue to the notcard is too slow! We will have to:
        //
        //  * move the IMU to another I2C bus and
        //  * run it in an interrupt (RTC alarm or EXTI/DRDY).
        //
        //  because:
        //
        //  * the notecard will block the I2C even if the IMU subsystem is running in an interrupt,
        //  * the notecard runs on 100kHz and IMU on 1mHz,
        //  * it takes longer to transmit a data package to the notecard than it takes for the IMU
        //    FIFO to fill up (using compressed FIFO might help, but reading the IMU would still be
        //    slow because we're locked at 100kHz).

        // Drain queue (if full enough)
        imu.check_drain_queue(&mut note, &mut delay).unwrap();


        // TODO:
        //
        // * We are now running as fast as we can: if this drains too much power then put
        // in some WFI + RTC or move IMU to other interrupt.
        //
        // * Set up and feed watchdog.
        //
        // * Handle and recover errors.
        //
        // * Does the notecard require more configuration to initiate sync? Maybe set up to
        //   sync every 10 or 20 minutes, depending on how full it is.
    }
}
