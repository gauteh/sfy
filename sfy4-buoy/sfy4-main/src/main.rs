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
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write},
};

#[cfg(feature = "storage")]
use embedded_hal::spi::MODE_0;
#[cfg(feature = "storage")]
use hal::spi::{Freq, Spi};

use git_version::git_version;
use hal::{
    gpio::{InterruptOpt, Mode},
    i2c,
    pac::interrupt,
};

use max_m10s::MaxM10S;
use rtcc::DateTimeAccess;

use sfy::gps::{EgpsTime, GpsCollector};
use sfy::log::log;
use sfy::note::Notecarrier;
use sfy::waves::Waves;
use sfy::{Imu, Location, SharedState, State, NOTEQ};

type GpsI2C = i2c::Iom2;
#[cfg(feature = "spectrum")]
use sfy::SPECQ;
#[cfg(feature = "storage")]
use sfy::{
    storage::{SdSpiSpeed, Storage},
    STORAGEQ,
};

mod log;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

type I = i2c::Iom3;
type E = <I as embedded_hal::blocking::i2c::Write>::Error;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub static COUNT: AtomicI32 = AtomicI32::new(0);
defmt::timestamp!("{=i32}", COUNT.load(Ordering::Relaxed));

/// The STATE contains the Real-Time-Clock which needs to be shared, as well as
/// up-to-date longitude and latitude.
pub static STATE: Mutex<RefCell<Option<SharedState<hal::rtc::Rtc>>>> =
    Mutex::new(RefCell::new(None));

/// GPS time snapshot for updating location / RTC on the main thread.
pub static EGPS_TIME: Mutex<RefCell<Option<EgpsTime>>> = Mutex::new(RefCell::new(None));

/// GPS timepulse interrupt pin (a2 = pad 11).
static TS_PIN: Mutex<RefCell<Option<hal::gpio::pin::Pin<11, { Mode::Input }>>>> =
    Mutex::new(RefCell::new(None));

/// IMU, moved into the RTC interrupt after setup.
static mut IMU: Option<Imu<E, I>> = None;

/// GPS driver + I2C bus + collector, moved into the GPIO interrupt after setup.
static mut GPS: Option<(MaxM10S, GpsI2C, GpsCollector)> = None;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[entry]
fn main() -> ! {
    unsafe {
        use core::ffi::c_void;

        halc::am_hal_clkgen_control(
            halc::am_hal_clkgen_control_e_AM_HAL_CLKGEN_CONTROL_SYSCLK_MAX,
            0 as *mut c_void,
        );
        halc::am_hal_cachectrl_config(&halc::am_hal_cachectrl_defaults);
        halc::am_hal_cachectrl_enable();
        halc::am_bsp_low_power_init();
    }

    let mut dp = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();
    let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

    let pins = hal::gpio::Pins::new(dp.GPIO);

    #[cfg(feature = "defmt-serial")]
    {
        use static_cell::StaticCell;
        static SERIAL: StaticCell<hal::uart::Uart0<48, 49>> = StaticCell::new();
        let serial = SERIAL.init(hal::uart::Uart0::new(dp.UART0, pins.tx0, pins.rx0));
        defmt_serial::defmt_serial(serial);
    }

    println!(
        "hello from sfy4 (v{}) (sn: {})!",
        git_version!(),
        sfy::note::BUOYSN
    );

    println!("firmware configuration:");
    println!("name ........: {}", sfy::note::BUOYSN);
    println!("notehub pr ..: {}", sfy::note::BUOYPR);
    println!("version .....: {}", git_version!());
    println!("storage .....: {}", cfg!(feature = "storage"));
    println!("fir .........: {}", cfg!(feature = "fir"));
    println!("raw .........: {}", cfg!(feature = "raw"));
    println!("20Hz ........: {}", cfg!(feature = "20Hz"));
    println!("continuous ..: {}", cfg!(feature = "continuous"));
    println!("cont-post ...: {}", cfg!(feature = "continuous-post"));
    println!("deploy ......: {}", cfg!(feature = "deploy"));
    println!("defmt-serial : {}", cfg!(feature = "defmt-serial"));
    println!("MAX-M10S GPS : true");
    println!("NOTEQ_SZ ....: {}", sfy::NOTEQ_SZ);
    println!("IMUQ_SZ .....: {}", sfy::IMUQ_SZ);
    println!("STORAGEQ_SZ .: {}", sfy::STORAGEQ_SZ);
    println!("EPGS_SZ .....: {}", sfy::EPGS_SZ);
    println!("GPS_PERIOD ..: {}", sfy::note::GPS_PERIOD);
    println!("GPS_HEARTBEAT: {}", sfy::note::GPS_HEARTBEAT);
    println!("SYNC_PERIOD .: {}", sfy::note::SYNC_PERIOD);
    println!("EXT_SIM_APN .: {}", sfy::note::EXT_APN);

    info!("Setting up IOM and RTC.");
    delay.delay_ms(1_000u32);

    // Power on the GPS module: d8 (pad 38), LOW = on.
    // NOTE: Must be configured BEFORE IOM4 init. Pads 38 and 39 share the same
    // Apollo3 config registers (PADREGJ, CFGE). Configuring pad 38 after IOM4
    // init would clobber pad 39's IOM4_SCL function selector.
    info!("Powering on GPS module..");
    let mut gps_pwr = pins.d8.into_push_pull_output();
    gps_pwr.set_low().unwrap();

    // IOM4: Notecarrier (100 kHz)
    let i2c4 = i2c::Iom4::new(dp.IOM4, pins.d10, pins.d9, i2c::Freq::F100kHz);
    // IOM3: IMU (400 kHz)
    let i2c3 = i2c::Iom3::new(dp.IOM3, pins.d6, pins.d7, i2c::Freq::F400kHz);
    // IOM2: MAX-M10S GPS — initialized later, right before first use.

    delay.delay_ms(200u32);

    // Set up RTC
    let mut rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
    rtc.set(
        &NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );
    rtc.enable();
    rtc.set_alarm_repeat(hal::rtc::AlarmRepeat::DeciSecond);
    rtc.enable_alarm();

    let mut location = Location::new();

    // Set up GPS timepulse interrupt on a2 (pad 11), rising edge.
    info!("Setting up GPS timepulse interrupt (a2 / pad 11)..");
    let mut ts = pins.a2.into_input();
    ts.configure_interrupt(InterruptOpt::LowToHigh);
    ts.clear_interrupt();
    // Arm after GPS is configured below.
    free(|cs| {
        TS_PIN.borrow(cs).replace(Some(ts));
    });

    let mut led = pins.d19.into_push_pull_output();

    info!("Blinking to indicate start-up.");
    led.set_high().unwrap();

    info!("Giving subsystems a couple of seconds to boot..");
    delay.delay_ms(5_000u32);
    led.set_low().unwrap();

    #[cfg(feature = "storage")]
    let storage = {
        info!("Setting up storage..");
        let spi = Spi::new(dp.IOM0, pins.d12, pins.d13, pins.d11, Freq::F100kHz, MODE_0);
        let cs = pins.a14.into_push_pull_output();
        let fdelay = hal::delay::FlashDelay;
        let mut storage = Storage::open(
            spi,
            cs,
            sfy::storage::clock::CountClock(&COUNT),
            |spi, speed| match speed {
                SdSpiSpeed::Low => spi.set_freq(Freq::F100kHz),
                SdSpiSpeed::High => spi.set_freq(Freq::F12mHz),
            },
            fdelay,
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

    #[cfg(feature = "storage")]
    let (imu_p, storage_consumer) = unsafe { STORAGEQ.split() };
    #[cfg(feature = "storage")]
    let (note_p, mut imu_queue) = unsafe { NOTEQ.split() };
    #[cfg(feature = "storage")]
    let mut storage_manager = sfy::StorageManager::new(storage, storage_consumer, note_p);

    #[cfg(not(feature = "storage"))]
    let (imu_p, mut imu_queue) = unsafe { NOTEQ.split() };

    info!("Setting up Notecarrier..");
    let mut note = Notecarrier::new(i2c4, &mut delay).unwrap();

    info!("Send startup-message over cellular.");
    let mut w = heapless::String::<100>::new();
    w.push_str("SFY4 (v").unwrap();
    w.push_str(git_version!()).unwrap();
    w.push_str(") (sn: ").unwrap();
    match sfy::note::BUOYSN {
        Some(sn) => w.push_str(sn).unwrap(),
        None => w.push_str("None").unwrap(),
    };
    w.push_str(") started up.").unwrap();
    info!("{}", w);

    note.hub()
        .log(&mut delay, w.as_str(), false, false)
        .and_then(|r| r.wait(&mut delay))
        .ok();

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
        now.map(|t| (t.and_utc().timestamp_millis() / 1000) as i32)
            .unwrap_or(0),
        Ordering::Relaxed,
    );
    info!(
        "Now: {:?} ms, position_time: {}, lat: {}, lon: {}",
        now.map(|t| t.and_utc().timestamp_millis()),
        position_time,
        lat,
        lon
    );

    info!("Setting up IMU..");
    let mut waves = Waves::new(i2c3).unwrap();
    waves
        .take_buf(
            now.map(|t| t.and_utc().timestamp_millis()).unwrap_or(0),
            position_time,
            lon,
            lat,
        )
        .unwrap();

    info!("Enable IMU.");
    waves.enable_fifo(&mut delay).unwrap();

    #[cfg(feature = "spectrum")]
    let (spec_p, mut spec_queue) = unsafe { SPECQ.split() };

    let imu = sfy::Imu::new(
        waves,
        imu_p,
        #[cfg(feature = "spectrum")]
        spec_p,
    );

    info!("Setting up MAX-M10S GPS over I2C..");
    // IOM2: initialized here, right before first use, to ensure the peripheral
    // is in a fresh idle state (IOM can be non-idle if initialized long before use).
    // Note: GPS was powered on early in startup; by this point several seconds have
    // elapsed (well above the ≥200 ms boot requirement).
    let mut i2c_gps = i2c::Iom2::new(dp.IOM2, pins.d17, pins.d18, i2c::Freq::F100kHz);
    let mut gnss = loop {
        match MaxM10S::new(&mut i2c_gps) {
            Ok(dev) => break dev,
            Err(_) => {
                warn!("GPS device not found — retrying");
                delay.delay_ms(500u32);
            }
        }
    };
    loop {
        match gnss.init(&mut i2c_gps) {
            Ok(()) => break,
            Err(e) => {
                warn!("GPS init failed: {:?} — retrying", defmt::Debug2Format(&e));
                delay.delay_ms(1000u32);
            }
        }
    }
    gnss.set_output_rate(&mut i2c_gps, 25)
        .inspect_err(|e| warn!("GPS set_output_rate failed: {:?}", defmt::Debug2Format(e)))
        .ok();
    loop {
        match gnss.set_pps_rate(&mut i2c_gps, 1_000_000, 100_000) {
            Ok(()) => {
                info!("GPS PPS configured: 1 Hz, 100 ms pulse");
                break;
            }
            Err(e) => {
                warn!(
                    "GPS set_pps_rate failed: {:?} — retrying",
                    defmt::Debug2Format(&e)
                );
                delay.delay_ms(500u32);
            }
        }
    }
    loop {
        match gnss.enable_pvt(&mut i2c_gps) {
            Ok(()) => break,
            Err(e) => {
                warn!(
                    "GPS enable_pvt failed: {:?} — retrying",
                    defmt::Debug2Format(&e)
                );
                delay.delay_ms(500u32);
            }
        }
    }
    info!("GPS initialised.");

    // Set up GPS packet collector.
    let (gps_p, mut gps_queue) = unsafe { sfy::gps::EGPSQ.split() };
    let mut gps_collector = GpsCollector::new(gps_p);

    // Move IMU and GPS into interrupt-accessible storage, arm timepulse interrupt.
    free(|cs| {
        unsafe {
            IMU = Some(imu);
            GPS = Some((gnss, i2c_gps, gps_collector));
        }
        if let Some(pin) = TS_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.enable_interrupt();
        }
    });

    defmt::info!("Enable interrupts");
    free(|_cs| unsafe {
        hal::gpio::enable_gpio_interrupts();
        cortex_m::interrupt::enable();
    });

    info!("Entering main loop");
    const GOOD_TRIES: u32 = 15;

    let mut last: i64 = 0;
    let mut good_tries: u32 = GOOD_TRIES;
    #[cfg(feature = "storage")]
    let mut sd_good: bool = true;

    loop {
        let now = STATE.now().map(|t| t.and_utc().timestamp_millis());
        defmt::debug!("main loop: now={}, last={}", now, last);

        // --- Drain storage queue to Notecard ----------------------------------
        #[cfg(feature = "storage")]
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
            Ok(Some(_)) => sd_good = true,
            _ => {}
        };

        const LOOP_DELAY: u32 = 1_000;
        const SHORT_LOOP_DELAY: u32 = 1_000;

        if ((now.unwrap_or(sfy::FUTURE.and_utc().timestamp_millis()) - last) > LOOP_DELAY as i64)
            || ((imu_queue.capacity() - imu_queue.len()) < 3
                && (now.unwrap_or(sfy::FUTURE.and_utc().timestamp_millis()) - last) > SHORT_LOOP_DELAY as i64)
        {
            let queue_time: f64 = f64::from(sfy::axl::SAMPLE_NO as u32)
                * f64::from(sfy::NOTEQ_SZ as u32)
                / f64::from(sfy::waves::OUTPUT_FREQ);
            debug_assert!(
                (f64::from(LOOP_DELAY) / 1000.) < queue_time,
                "loop is too slow, NOTEQ will overflow: loop: {} ms vs queue: {} ms",
                LOOP_DELAY,
                queue_time * 1000.
            );

            // Apply GPS time/position to RTC and location state.
            location.set_from_egps(&STATE, &EGPS_TIME);

            #[cfg(feature = "storage")]
            defmt::warn!(
                "notecard iteration, now: {}, note queue: {}, storage queue: {}",
                now,
                imu_queue.len(),
                storage_manager.storage_queue.len()
            );

            #[cfg(not(feature = "storage"))]
            defmt::warn!(
                "notecard iteration, now: {}, note queue: {}, gps queue: {}",
                now,
                imu_queue.len(),
                gps_queue.len(),
            );

            #[cfg(not(feature = "deploy"))]
            led.toggle().unwrap();

            sfy::log::drain_log(&mut note, &mut delay)
                .inspect_err(|e| defmt::error!("drain log: {:?}", e))
                .ok();

            let nd = note.drain_queue(&mut imu_queue, &mut delay);
            let ng = note.drain_egps_queue(&mut gps_queue, &mut delay);

            #[cfg(feature = "spectrum")]
            note.drain_spec_queue(&mut spec_queue, &mut delay)
                .inspect_err(|e| defmt::error!("drain spec queue: {:?}", e))
                .ok();

            let ns = note.check_and_sync(&mut delay);

            match (nd, ng, ns) {
                (Ok(_), Ok(_), Ok(_)) => good_tries = GOOD_TRIES,
                (dq, dg, cs) => {
                    error!(
                        "Fatal error in main loop: drain_queue: {:?}, drain_egps_queue: {:?}, check_and_sync: {:?}. Tries left: {}",
                        dq,
                        dg,
                        cs,
                        good_tries
                    );

                    delay.delay_ms(100u16);
                    note.reset(&mut delay).ok();
                    delay.delay_ms(100u16);

                    let mut msg = heapless::String::<512>::new();
                    write!(
                        &mut msg,
                        "Fatal error in main loop: drain_queue: {:?}, check_and_sync: {:?}. Tries left: {}",
                        dq, cs, good_tries
                    )
                    .inspect_err(|e| {
                        defmt::error!(
                            "failed to format error: {:?}",
                            defmt::Debug2Format(e)
                        )
                    })
                    .ok();

                    warn!("Trying to send log message..");
                    note.hub()
                        .log(&mut delay, &msg, false, false)
                        .and_then(|f| f.wait(&mut delay))
                        .ok();

                    if good_tries == 0 {
                        error!("No more tries left, resetting.");
                        reset(&mut note, &mut delay);
                    } else {
                        good_tries -= 1;
                    }
                }
            };

            last = now.unwrap_or(0);
        }

        #[cfg(not(feature = "deploy"))]
        delay.delay_ms(1000u16);

        #[cfg(feature = "deploy")]
        asm::wfi();
    }
}

fn reset<I: Read + Write>(note: &mut Notecarrier<I>, delay: &mut impl DelayMs<u16>) -> ! {
    cortex_m::interrupt::disable();
    warn!("Resetting device!");
    note.reset(delay).ok();
    info!("Trying to send any remaining log messages..");
    sfy::log::drain_log(note, delay).ok();
    warn!("Resetting in 3 seconds..");
    delay.delay_ms(3_000u16);
    cortex_m::peripheral::SCB::sys_reset()
}

// ---------------------------------------------------------------------------
// GPIO interrupt — GPS timepulse (a2 / pad 11, rising edge)
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
#[interrupt]
fn GPIO() {
    static mut gps: Option<(MaxM10S, GpsI2C, GpsCollector)> = None;

    // On first call: move GPS out of static storage into ISR-local storage.
    if gps.is_none() {
        *gps = unsafe { GPS.take() };
    }

    // Capture the RTC time at the moment of the timepulse and clear the pin interrupt.
    let pps_time = free(|cs| {
        let pps_time = {
            let mut state = STATE.borrow(cs).borrow_mut();
            if let Some(state) = state.as_mut() {
                state
                    .rtc
                    .datetime()
                    .ok()
                    .map(|t| t.and_utc().timestamp_millis())
                    .unwrap_or(0)
            } else {
                0
            }
        };
        defmt::info!("GPS timepulse: pps_time = {}", pps_time);

        if let Some(pin) = TS_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.clear_interrupt();
            pin.enable_interrupt();
        }

        pps_time
    });

    // Read the PVT sample and push it into the collector.
    //
    // The timepulse fires at the GPS epoch boundary; the navigation engine
    // outputs NAV-PVT asynchronously after the solution is computed (~100–500 ms
    // later at 1 Hz).  A short busy-wait here ensures the message has arrived in
    // the DDC FIFO before we poll it.  pps_time was already captured above so
    // this delay does not affect RTC timing accuracy.
    hal::delay::FlashDelay::delay_us(100_000); // 100 ms

    if let Some((gnss, i2c_gps, collector)) = gps {
        match gnss.read_pvt(i2c_gps) {
            Ok(Some(pvt)) => {
                if let Some(egps) = EgpsTime::from_pvt(&pvt, pps_time) {
                    free(|cs| {
                        EGPS_TIME.borrow(cs).replace(Some(egps));
                    });
                }
                collector.add_sample(pvt);
            }
            Ok(None) => {
                defmt::debug!("GPS timepulse fired but no NAV-PVT in FIFO yet");
            }
            Err(e) => {
                defmt::error!("GPS read_pvt error: {:?}", defmt::Debug2Format(&e));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RTC interrupt — IMU sampling
// ---------------------------------------------------------------------------

#[cfg(not(feature = "host-tests"))]
#[allow(non_snake_case)]
#[interrupt]
fn RTC() {
    static mut imu: Option<Imu<E, I>> = None;
    static mut GOOD_TRIES: u16 = 5;

    // Clear RTC interrupt
    unsafe {
        (*(hal::pac::RTC::ptr()))
            .intclr
            .write(|w| w.alm().set_bit());
    }

    if let Some(imu) = imu {
        let (now, position_time, lon, lat) = if let Some((now, position_time, lon, lat)) =
            free(|cs| {
                let mut state = STATE.borrow(cs).borrow_mut();
                state.as_mut().map(|state| {
                    let now = state.rtc.datetime().ok();
                    let now = now.map(|t| t.and_utc().timestamp_millis()).unwrap_or(0);
                    (now, state.position_time, state.lon, state.lat)
                })
            }) {
            (now, position_time, lon, lat)
        } else {
            error!("RTC: failed, skipping RTC interrupt.");
            return;
        };

        COUNT.store((now / 1000) as i32, Ordering::Relaxed);

        match imu.check_retrieve(now, position_time, lon, lat) {
            Ok(_) => {
                *GOOD_TRIES = 5;
            }
            Err(e) => {
                error!("RTC: IMU check_retrieve failed: {:?}", e);
                if *GOOD_TRIES == 0 {
                    error!("RTC: too many IMU failures, resetting.");
                    cortex_m::peripheral::SCB::sys_reset();
                }
                *GOOD_TRIES -= 1;
            }
        }
    } else {
        // First call: move IMU out of the static storage.
        *imu = unsafe { IMU.take() };
    }
}

// ---------------------------------------------------------------------------
// Fault handlers
// ---------------------------------------------------------------------------

#[cfg(not(feature = "deploy"))]
#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();

    defmt::error!("panic: {}", defmt::Display2Format(info));

    let mut msg = heapless::String::<512>::new();
    write!(&mut msg, "panic: {}", info)
        .inspect_err(|e| defmt::error!("failed to format panic: {:?}", defmt::Debug2Format(e)))
        .ok();
    log(&msg);

    unsafe {
        if let Some(note) = log::NOTE {
            sfy::log::drain_log(&mut *note, &mut hal::delay::FlashDelay).ok();
        }
    }

    loop {
        cortex_m::asm::bkpt();
    }
}

#[cfg(feature = "deploy")]
#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();

    defmt::error!("panic: {}", defmt::Display2Format(info));

    let mut msg = heapless::String::<512>::new();
    write!(&mut msg, "panic: {}", info)
        .inspect_err(|e| defmt::error!("failed to format panic: {:?}", defmt::Debug2Format(e)))
        .ok();
    log(&msg);

    unsafe {
        if let Some(note) = log::NOTE {
            sfy::log::drain_log(&mut *note, &mut hal::delay::FlashDelay).ok();
        }
    }

    cortex_m::peripheral::SCB::sys_reset()
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    defmt::error!("HardFault: {:#?}", defmt::Debug2Format(ef));

    let mut msg = heapless::String::<512>::new();
    write!(&mut msg, "HardFault: {:?}", ef)
        .inspect_err(|e| defmt::error!("failed to format hard fault: {:?}", defmt::Debug2Format(e)))
        .ok();
    log(&msg);

    unsafe {
        if let Some(note) = log::NOTE {
            sfy::log::drain_log(&mut *note, &mut hal::delay::FlashDelay).ok();
        }
    }

    cortex_m::peripheral::SCB::sys_reset()
}
