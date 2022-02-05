#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(all(not(test), not(feature = "deploy")))]
use panic_probe as _;

#[cfg(feature = "deploy")]
use panic_reset as _;

#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};

use ambiq_hal::{self as hal, prelude::*};
use chrono::NaiveDate;
use core::cell::RefCell;
#[allow(unused_imports)]
use cortex_m::{
    asm,
    interrupt::{free, Mutex},
};
use cortex_m_rt::{entry, exception, ExceptionFrame};
use defmt_rtt as _;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write},
};
use git_version::git_version;
use hal::{i2c, pac::interrupt};
use heapless::mpmc::Q16;

use sfy::note::Notecarrier;
use sfy::waves::Waves;
use sfy::{Imu, Location, SharedState, State};

/// Log message queue for messages to be sent back over notecard.
static LOGQ: Q16<heapless::String<24>> = Q16::new();


/// This queue is filled up by the IMU in an interrupt with ready batches of time series. It is
/// consumed by the main thread and drained to the notecard / cellular.
static mut IMUQ: heapless::spsc::Queue<sfy::axl::AxlPacket, 32> = heapless::spsc::Queue::new();

/// This static is only used to transfer ownership of the IMU subsystem to the interrupt handler.
type I = hal::i2c::Iom3;
type E = <I as embedded_hal::blocking::i2c::Write>::Error;
static mut IMU: Option<sfy::Imu<E, I>> = None;

/// The STATE contains the Real-Time-Clock which needs to be shared, as well as up-to-date
/// longitude and latitude.
static STATE: Mutex<RefCell<Option<SharedState>>> = Mutex::new(RefCell::new(None));

#[cfg_attr(not(test), entry)]
fn main() -> ! {
    println!("hello from sfy (v{}) (sn: {})!", git_version!(), sfy::note::BUOYSN);

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

    let mut dp = defmt::unwrap!(hal::pac::Peripherals::take());
    let core = defmt::unwrap!(hal::pac::CorePeripherals::take());
    let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

    let pins = hal::gpio::Pins::new(dp.GPIO);
    let mut led = pins.d19.into_push_pull_output(); // d14 on redboard_artemis

    let i2c2 = i2c::I2c::new(dp.IOM2, pins.d17, pins.d18, i2c::Freq::F100kHz);
    let i2c3 = i2c::I2c::new(dp.IOM3, pins.d6, pins.d7, i2c::Freq::F1mHz);

    // Set up RTC
    let mut rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
    rtc.set(NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0)); // Now timestamps will be positive.
    rtc.enable();
    rtc.set_alarm_repeat(hal::rtc::AlarmRepeat::CentiSecond);
    rtc.enable_alarm();

    let mut location = Location::new();

    info!("Giving subsystems a couple of seconds to boot..");
    delay.delay_ms(5_000u32);

    info!("Setting up Notecarrier..");
    let mut note = defmt::unwrap!(Notecarrier::new(i2c2, &mut delay));

    info!("Send startup-message over cellular.");

    let mut w = heapless::String::<100>::new();
    defmt::unwrap!(w.push_str("SFY (v"));
    defmt::unwrap!(w.push_str(git_version!()));
    defmt::unwrap!(w.push_str(") (sn: "));
    defmt::unwrap!(w.push_str(sfy::note::BUOYSN));
    defmt::unwrap!(w.push_str(") started up."));
    info!("{}", w);

    LOGQ.enqueue(heapless::String::from("SFY startup")).ok();

    note.hub()
        .log(w.as_str(), false, false)
        .and_then(|r| r.wait(&mut delay))
        .ok(); // this will fail if more than 100 notes is added.

    info!("Setting up IMU..");
    let mut waves = defmt::unwrap!(Waves::new(i2c3));
    defmt::unwrap!(waves.take_buf(rtc.now().timestamp_millis(), 0.0, 0.0)); // set timestamp.

    info!("Enable IMU.");
    defmt::unwrap!(waves.enable_fifo(&mut delay));

    let imu = sfy::Imu::new(waves, unsafe { IMUQ.split().0 });
    let mut imu_queue = unsafe { IMUQ.split().1 };

    unsafe { IMU = Some(imu) };

    free(|cs| {
        STATE.borrow(cs).replace(Some(SharedState {
            rtc,
            lon: 0.0,
            lat: 0.0,
        }));
    });

    defmt::info!("Enable interrupts");
    unsafe {
        cortex_m::interrupt::enable();
    }

    info!("Entering main loop");
    const GOOD_TRIES: u32 = 5;

    let mut last: i64 = 0;
    let mut good_tries: u32 = GOOD_TRIES;

    loop {
        let now = STATE.now().timestamp_millis();

        note.drain_log(&LOGQ, &mut delay).ok();

        if (now - last) > 1000 {
            defmt::debug!("iteration, now: {}..", now);
            defmt::unwrap!(led.toggle());
            match (
                location.check_retrieve(&STATE, &mut delay, &mut note),
                note.drain_queue(&mut imu_queue, &mut delay),
                note.check_and_sync(&mut delay),
            ) {
                (Ok(_), Ok(_), Ok(_)) => good_tries = GOOD_TRIES,
                (l, dq, cs) => {
                    error!(
                        "Fatal error occured during main loop: location: {:?}, note/drain_queue: {:?}, note/check_and_sync: {:?}. Tries left: {}",
                        l,
                        dq,
                        cs,
                        good_tries
                    );

                    if good_tries == 0 {
                        error!("No more tries left, attempting to reset devices and restart.");
                        reset(&mut note, &mut delay);
                    } else {
                        good_tries -= 1;
                    }
                }
            };
            last = now;
        }

        #[cfg(not(feature = "deploy"))]
        delay.delay_ms(1000u16);

        #[cfg(feature = "deploy")]
        asm::wfi(); // doesn't work very well with RTT + probe

        // defmt::flush();

        // TODO:
        // * Set up and feed watchdog.
    }
}

fn reset<I: Read + Write>(note: &mut Notecarrier<I>, delay: &mut impl DelayMs<u16>) -> ! {
    cortex_m::interrupt::disable();

    warn!("Resetting device!");

    debug!("notecard: consuming any remaining response.");
    unsafe { note.consume_response(delay).ok() };

    info!("Trying to send any remaining log messages..");
    note.drain_log(&LOGQ, delay).ok();

    warn!("Trying to send log message..");
    note.hub()
        .log("Error occured in main loop: restarting.", false, false)
        .and_then(|f| f.wait(delay))
        .ok();

    warn!("Trying to restart notecard..");
    note.card()
        .restart()
        .and_then(|f| f.wait(delay))
        .and_then(|r| {
            info!("Notecard succesfully restarted.");
            Ok(r)
        })
        .or_else(|e| {
            error!("Could not restart notecard.");
            Err(e)
        })
        .ok();

    warn!("Resetting in 3 seconds..");
    delay.delay_ms(3_000u16);

    cortex_m::peripheral::SCB::sys_reset()
}

#[cfg(not(feature = "host-tests"))]
#[allow(non_snake_case)]
#[interrupt]
fn RTC() {
    #[allow(non_upper_case_globals)]
    static mut imu: Option<Imu<E, I>> = None;
    static mut GOOD_TRIES: u16 = 5;

    // FIFO size of IMU is 512 samples (uncompressed), sample rate at IMU is 208 Hz. So we
    // need to empty FIFO at atleast (512 / 208) Hz = 2.46 Hz or every 404 ms.

    // Clear RTC interrupt
    unsafe {
        (*(hal::pac::RTC::ptr()))
            .intclr
            .write(|w| w.alm().set_bit());
    }

    if let Some(imu) = imu {
        let (now, lon, lat) = free(|cs| {
            let state = STATE.borrow(cs).borrow();
            let state = defmt::unwrap!(state.as_ref());

            let now = state.rtc.now().timestamp_millis();
            let lon = state.lon;
            let lat = state.lat;

            (now, lon, lat)
        });

        // XXX: This is the most time-critical part of the program.
        //
        // It seems that the IMU I2C communication sometimes fails with a NAK, causing a module
        // reset, which again might cause a HardFault.
        match imu.check_retrieve(now, lon, lat) {
            Ok(_) => {
                *GOOD_TRIES = 5;
            }
            Err(e) => {
                error!("IMU ISR failed: {:?}", e);
                LOGQ.enqueue(heapless::String::from("IMU ISR failed")).ok();

                if *GOOD_TRIES == 0 {
                    error!("IMU has failed repatedly, resetting system.");
                    cortex_m::peripheral::SCB::sys_reset();
                }

                *GOOD_TRIES -= 1;
            }
        }
    } else {
        unsafe {
            imu.replace(defmt::unwrap!(IMU.take()));
        }
    }
}

#[cfg(not(feature = "host-tests"))]
#[allow(non_snake_case)]
#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    error!("hard fault exception: {:#?}", defmt::Debug2Format(ef));

    warn!("resetting system..");
    cortex_m::peripheral::SCB::sys_reset()
}
