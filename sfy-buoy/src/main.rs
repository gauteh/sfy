#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use panic_probe as _; // TODO: Restart board on panic.

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

use cortex_m;
use cortex_m_rt::entry;
use defmt_rtt as _;
use ambiq_hal::{self as hal, prelude::*};

use sfy::note::Notecarrier;

#[entry]
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

    let serial = hal::uart::Uart0::new(dp.UART0, pins.tx0, pins.rx0);

    defmt::println!("hello sfy!");

    // Initialize notecard
    let i2c = hal::i2c::I2c::new(dp.IOM2, pins.d17, pins.d18);
    let mut note = Notecard::new(i2c);
    note.initialize().expect("could not initialize notecard.");

    defmt::info!("Waiting to start..!");
    delay.delay_ms(2000u32);

    defmt::info!("hello world!");

    // Delay
    delay.delay_ms(300u32);

    if note.ping() {
        warn!("notecard found!");
    } else {
        error!("notecard not found!");
    }

    info!("note: card.time");
    info!("note: time: {:?}", note.card().time().unwrap().wait());

    note.hub()
        .set(Some("com.vetsj.gaute.eg:sby"), None, None, Some("cain"))
        .unwrap()
        .wait()
        .ok();

    delay.delay_ms(1000u32);
    info!("done: looping.");

    warn!("note: logging startup");
    note.hub().log("cain starting up!", true, true).unwrap().wait().ok();

    info!("set note in periodic tracker mode");
    debug!("mode: {:?}", note.card().location_mode(Some("periodic"), Some(60), None, None, None, None, None, None).unwrap().wait());
    debug!("track: {:?}", note.card().location_track(true, false, true, None, None).unwrap().wait());


    warn!("note: syncing");
    note.hub().sync().unwrap().wait().ok();

    let mut i = 0;

    loop {
        delay.delay_ms(2000u32);
        info!("note: card.time");
        info!("note: time: {:?}", note.card().time().unwrap().wait());

        info!("querying status..");
        info!("status: {:?}", note.card().status().unwrap().wait());

        info!("querying sync status..");
        info!("status: {:?}", note.hub().sync_status().unwrap().wait());

        // Toggle LEDs
        led.toggle().unwrap();

        if i % 10 == 0 {
            warn!("track: {:?}", note.card().location_track(true, false, true, None, None).unwrap().wait());
            note.hub().sync().unwrap().wait().ok();
        }

        debug!("location mode: {:?}", note.card().location_mode(Some(""), None, None, None, None, None, None, None).unwrap().wait());

        i += 1;
    }
}

