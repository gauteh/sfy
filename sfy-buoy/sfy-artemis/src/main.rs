#![feature(result_option_inspect)]
#![no_std]
#![no_main]

#[cfg(not(feature = "deploy"))]
use panic_probe as _;

#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};

#[cfg(not(feature = "defmt-serial"))]
use defmt_rtt as _;

#[cfg(feature = "defmt-serial")]
use defmt_serial as _;

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

use ambiq_hal::{self as hal, prelude::*};
use chrono::NaiveDate;
use core::cell::RefCell;
use core::fmt::Write as _;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicI32, Ordering};
#[allow(unused_imports)]
use cortex_m::{
    asm,
    interrupt::{free, Mutex},
};
use cortex_m_rt::{entry, exception, ExceptionFrame};
use embedded_hal::{
    blocking::{
        delay::DelayMs,
        i2c::{Read, Write},
    },
    spi,
};
use git_version::git_version;
use hal::spi::{Freq, Spi};
use hal::{i2c, pac::interrupt};

use sfy::log::log;
use sfy::note::Notecarrier;
use sfy::waves::Waves;
use sfy::{
    storage::{SdSpiSpeed, Storage},
    STORAGEQ,
};
use sfy::{Imu, Location, SharedState, State, NOTEQ};

mod log;

/// This static is used to transfer ownership of the IMU subsystem to the interrupt handler.
type I = hal::i2c::Iom3;
type E = <I as embedded_hal::blocking::i2c::Write>::Error;
static mut IMU: Option<sfy::Imu<E, I>> = None;

pub static COUNT: AtomicI32 = AtomicI32::new(0);
defmt::timestamp!("{=i32}", COUNT.load(Ordering::Relaxed));

/// The STATE contains the Real-Time-Clock which needs to be shared, as well as up-to-date
/// longitude and latitude.
pub static STATE: Mutex<RefCell<Option<SharedState<hal::rtc::Rtc>>>> =
    Mutex::new(RefCell::new(None));

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

    // set up serial as defmt target.
    #[cfg(feature = "defmt-serial")]
    let serial = hal::uart::Uart0::new(dp.UART0, pins.tx0, pins.rx0);
    #[cfg(feature = "defmt-serial")]
    defmt_serial::defmt_serial(serial);

    println!(
        "hello from sfy (v{}) (sn: {})!",
        git_version!(),
        sfy::note::BUOYSN
    );

    info!("Setting up IOM and RTC.");
    delay.delay_ms(1_000u32);

    let i2c4 = i2c::I2c::new(dp.IOM4, pins.d10, pins.d9, i2c::Freq::F100kHz);
    let i2c3 = i2c::I2c::new(dp.IOM3, pins.d6, pins.d7, i2c::Freq::F1mHz);

    // Set up RTC
    let mut rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
    rtc.set(&NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0)); // Now timestamps will be positive.
    rtc.enable();
    rtc.set_alarm_repeat(hal::rtc::AlarmRepeat::CentiSecond);
    rtc.enable_alarm();

    let mut location = Location::new();

    info!("Giving subsystems a couple of seconds to boot..");
    delay.delay_ms(5_000u32);

    let storage = {
        info!("Setting up storage..");

        debug!("Setting up SPI for SD card..");
        let spi = Spi::new(
            dp.IOM0,
            pins.d12,
            pins.d13,
            pins.d11,
            Freq::F100kHz,
            spi::MODE_0,
        );
        let cs = pins.a14.into_push_pull_output();

        let mut storage = Storage::open(
            spi,
            cs,
            sfy::storage::clock::CountClock(&COUNT),
            |spi, speed| match speed {
                SdSpiSpeed::Low => spi.set_freq(Freq::F100kHz),
                SdSpiSpeed::High => spi.set_freq(Freq::F12mHz),
            },
        );

        storage
            .acquire()
            .inspect_err(|e| {
                defmt::error!("Failed to setup storage: {}", e);

                let mut msg = heapless::String::<256>::new();
                write!(&mut msg, "storage setup err: {:?}", e)
                    .inspect_err(|e| {
                        defmt::error!("failed to format storage-err: {:?}", defmt::Debug2Format(e))
                    })
                    .ok();
                log(&msg);
            })
            .ok();

        storage
    };

    let (imu_p, storage_consumer) = unsafe { STORAGEQ.split() };
    let (note_p, mut imu_queue) = unsafe { NOTEQ.split() };

    let mut storage_manager = sfy::StorageManager::new(storage, storage_consumer, note_p);

    info!("Setting up Notecarrier..");
    let mut note = Notecarrier::new(i2c4, &mut delay).unwrap();

    info!("Send startup-message over cellular.");

    let mut w = heapless::String::<100>::new();
    w.push_str("SFY (v").unwrap();
    w.push_str(git_version!()).unwrap();
    w.push_str(") (sn: ").unwrap();
    w.push_str(sfy::note::BUOYSN).unwrap();
    w.push_str(") started up.").unwrap();
    info!("{}", w);

    note.hub()
        .log(&mut delay, w.as_str(), false, false)
        .and_then(|r| r.wait(&mut delay))
        .ok(); // this will fail if more than 100 notes is added.

    // Move state into globally available variables and set reference to NOTE for
    // logging on panic and hard resets.
    //
    // TODO: Should maybe `pin_mut!` NOTE to prevent it being moved on the stack.
    free(|cs| unsafe {
        log::NOTE = Some(&mut note as *mut _);

        STATE.borrow(cs).replace(Some(SharedState {
            rtc,
            position_time: 0,
            lon: 0.0,
            lat: 0.0,
        }));
    });

    info!("Try to fetch location and time before starting main loop..");
    location
        .check_retrieve(&STATE, &mut delay, &mut note)
        .inspect_err(|e| error!("Failed retrieving location and time: {:?}", e))
        .ok();

    let (now, position_time, lat, lon) = STATE.get();
    COUNT.store(
        (now.timestamp_millis() / 1000).try_into().unwrap_or(0),
        Ordering::Relaxed,
    );
    info!(
        "Now: {} ms, position_time: {}, lat: {}, lon: {}",
        now.timestamp_millis(),
        position_time,
        lat,
        lon
    );

    info!("Setting up IMU..");
    let mut waves = Waves::new(i2c3).unwrap();
    waves
        .take_buf(now.timestamp_millis(), position_time, lon, lat)
        .unwrap(); // set timestamp.

    info!("Enable IMU.");
    waves.enable_fifo(&mut delay).unwrap();

    let imu = sfy::Imu::new(waves, imu_p);

    // Move IMU into temporary variable for moving it into the `RTC` interrupt
    // routine, _before_ we enable interrupts.
    free(|_| {
        unsafe { IMU = Some(imu) };
    });

    defmt::info!("Enable interrupts");
    unsafe {
        cortex_m::interrupt::enable();
    }

    info!("Entering main loop");
    const GOOD_TRIES: u32 = 15;

    let mut last: i64 = 0;
    let mut good_tries: u32 = GOOD_TRIES;
    let mut sd_good: bool = true; // Do not spam with log messags.

    loop {
        let now = STATE.now().timestamp_millis();

        let l = location.check_retrieve(&STATE, &mut delay, &mut note);

        // Move data to SD card and enqueue for Notecard.
        match storage_manager.drain_queue(&mut note, &mut delay) {
            Err(e) => {
                error!("Failed to write to SD card: {:?}", e);

                if sd_good {
                    let mut msg = heapless::String::<256>::new();
                    write!(&mut msg, "storage-err-l: {:?}", e)
                        .inspect_err(|e| {
                            defmt::error!(
                                "failed to format storage-err: {:?}",
                                defmt::Debug2Format(e)
                            )
                        })
                        .ok();
                    log(&msg);
                }

                sd_good = false;
            }
            Ok(Some(_)) => {
                sd_good = true;
            }
            _ => {}
        };

        // Process data and communication for the Notecard.
        if imu_queue.ready() || ((now - last) > 5000) {
            defmt::debug!("notecard iteration, now: {}, imu queue: {}", now, imu_queue.len());
            led.toggle().unwrap();

            sfy::log::drain_log(&mut note, &mut delay)
                .inspect_err(|e| defmt::error!("drain log: {:?}", e))
                .ok();


            let nd = note.drain_queue(&mut imu_queue, &mut delay);
            let ns = note.check_and_sync(&mut delay);

            match (l, nd, ns) {
                (Ok(_), Ok(_), Ok(_)) => good_tries = GOOD_TRIES,
                (l, dq, cs) => {
                    error!(
                        "Fatal error occured during main loop: location: {:?}, note/drain_queue: {:?}, note/check_and_sync: {:?}. Tries left: {}",
                        l,
                        dq,
                        cs,
                        good_tries
                    );

                    // Notecard might be in WrongState.
                    delay.delay_ms(100u16);
                    note.reset(&mut delay).ok();
                    delay.delay_ms(100u16);

                    let mut msg = heapless::String::<512>::new();
                    write!(&mut msg, "Fatal error in main loop: location: {:?}, note/drain_queue: {:?}, note/check_and_sync: {:?}. Tries left: {}", l, dq, cs, good_tries)
                        .inspect_err(|e| defmt::error!("failed to format error: {:?}", defmt::Debug2Format(e)))
                        .ok();

                    warn!("Trying to send log message..");
                    note.hub()
                        .log(&mut delay, &msg, false, false)
                        .and_then(|f| f.wait(&mut delay))
                        .ok();

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
        if !(imu_queue.ready() || storage_manager.storage_queue.ready()) {
            asm::wfi(); // doesn't work very well with RTT + probe
        }

        // defmt::flush();

        // TODO:
        // * Set up and feed watchdog.
    }
}

fn reset<I: Read + Write>(note: &mut Notecarrier<I>, delay: &mut impl DelayMs<u16>) -> ! {
    cortex_m::interrupt::disable();

    warn!("Resetting device!");

    debug!("notecard: consuming any remaining response.");
    note.reset(delay).ok();

    info!("Trying to send any remaining log messages..");
    sfy::log::drain_log(note, delay).ok();

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
    // need to empty FIFO at atleast (208 / 512) Hz = 0.406 Hz or every 2.46 s.

    // Clear RTC interrupt
    unsafe {
        (*(hal::pac::RTC::ptr()))
            .intclr
            .write(|w| w.alm().set_bit());
    }

    if let Some(imu) = imu {
        let (now, position_time, lon, lat) = free(|cs| {
            let state = STATE.borrow(cs).borrow();
            let state = state.as_ref().unwrap();

            let now = state.rtc.now().timestamp_millis();
            let position_time = state.position_time;
            let lon = state.lon;
            let lat = state.lat;

            (now, position_time, lon, lat)
        });

        COUNT.store((now / 1000).try_into().unwrap_or(0), Ordering::Relaxed);

        // XXX: This is the most time-critical part of the program.
        //
        // It seems that the IMU I2C communication sometimes fails with a NAK, causing a module
        // reset, which again might cause a HardFault.
        match imu.check_retrieve(now, position_time, lon, lat) {
            Ok(_) => {
                *GOOD_TRIES = 5;
            }
            Err(e) => {
                error!("IMU ISR failed: {:?}, resetting IMU..", e);

                let mut delay = hal::delay::FlashDelay;

                let r = imu.reset(now, position_time, lon, lat, &mut delay);
                warn!("IMU reset: {:?}", r);

                let mut msg = heapless::String::<256>::new();
                write!(&mut msg, "IMU failure: {:?}, reset: {:?}", e, r)
                    .inspect_err(|e| {
                        defmt::error!("failed to format IMU failure: {:?}", defmt::Debug2Format(e))
                    })
                    .ok();
                log(&msg);

                if *GOOD_TRIES == 0 {
                    panic!("IMU has failed repeatedly: {:?}, resetting system.", e);
                }

                *GOOD_TRIES -= 1;
            }
        }
    } else {
        unsafe {
            imu.replace(IMU.take().unwrap());
        }
    }
}

#[allow(non_snake_case)]
#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    error!(
        "hard fault exception: {:#?}. resetting system.",
        defmt::Debug2Format(ef)
    );
    cortex_m::peripheral::SCB::sys_reset()
}

#[cfg(feature = "deploy")]
#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    defmt::error!("panic: {}", defmt::Debug2Format(info));
    log("panic reset.");
    let mut msg = heapless::String::<256>::new();
    write!(&mut msg, "panic: {}", info)
        .inspect_err(|e| defmt::error!("failed to format panic: {:?}", defmt::Debug2Format(e)))
        .ok();
    log(&msg);

    let mut delay = hal::delay::FlashDelay;

    free(|_| unsafe { sfy::log::panic_drain_log(log::NOTE, &mut delay) });

    defmt::error!("panic logged, resetting..");
    cortex_m::peripheral::SCB::sys_reset();
}
