#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use panic_probe as _; // TODO: Restart board on panic.

#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};

use ambiq_hal::{self as hal, prelude::*};
use chrono::NaiveDate;
use core::cell::RefCell;
use cortex_m::{
    asm,
    interrupt::{free, Mutex},
};
#[cfg(not(test))]
use cortex_m_rt::entry;
use defmt_rtt as _;
use hal::{i2c, pac::interrupt};

use sfy::note::Notecarrier;
use sfy::waves::Waves;
use sfy::{Imu, Location, SharedState};

/// This queue is filled up by the IMU in an interrupt with ready batches of time series. It is
/// consumed by the main thread and drained to the notecard / cellular.
static mut IMUQ: heapless::spsc::Queue<sfy::note::AxlPacket, 16> = heapless::spsc::Queue::new();

/// This static is only used to transfer ownership of the IMU subsystem to the interrupt handler.
type I = shared_bus::I2cProxy<'static, shared_bus::CortexMMutex<hal::i2c::Iom2>>;
type E = <I as embedded_hal::blocking::i2c::Write>::Error;
static mut IMU: Option<sfy::Imu<E, I>> = None;

/// The STATE contains the Real-Time-Clock which needs to be shared, as well as up-to-date
/// longitude and latitude.
static STATE: Mutex<RefCell<Option<SharedState>>> = Mutex::new(RefCell::new(None));

#[cfg_attr(not(test), entry)]
fn main() -> ! {
    println!("hello from sfy!");

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
    // let bus = shared_bus::BusManagerSimple::new(i2c);
    let bus: &'static _ = shared_bus::new_cortexm!(hal::i2c::Iom2 = i2c).unwrap();

    // Set up RTC
    let mut rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
    rtc.set(NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0)); // Now timestamps will be positive.
    rtc.enable();
    rtc.set_alarm_repeat(hal::rtc::AlarmRepeat::SEC);
    rtc.enable_alarm();

    let mut location = Location::new();

    info!("Setting up Notecarrier..");
    let mut note = Notecarrier::new(bus.acquire_i2c(), &mut delay).unwrap();

    info!("Setting up IMU..");
    let mut waves = Waves::new(bus.acquire_i2c()).unwrap();
    waves
        .take_buf(rtc.now().timestamp_millis() as u32, 0.0, 0.0)
        .unwrap(); // set timestamp.

    info!("Enable IMU.");
    waves.enable_fifo(&mut delay).unwrap();

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
    loop {
        defmt::debug!("iteration..");
        led.toggle().unwrap();

        location.check_retrieve(&STATE, &mut delay, &mut note).unwrap();
        note.drain_queue(&mut imu_queue, &mut delay).unwrap();

        asm::wfi();

        // TODO:
        // * Set up and feed watchdog.
        // * Handle and recover errors.
    }
}

#[cfg(not(feature = "host-tests"))]
#[allow(non_snake_case)]
#[interrupt]
fn RTC() {
    #[allow(non_upper_case_globals)]
    static mut imu: Option<Imu<E, I>> = None;

    defmt::debug!("RTC interrupt");

    // Clear RTC interrupt
    unsafe {
        (*(hal::pac::RTC::ptr()))
            .intclr
            .write(|w| w.alm().set_bit());
    }

    if let Some(imu) = imu {
        let (now, lon, lat) = free(|cs| {
            let state = STATE.borrow(cs).borrow();
            let state = state.as_ref().unwrap();

            let now = state.rtc.now().timestamp_millis();
            let lon = state.lon;
            let lat = state.lat;

            (now, lon, lat)
        });

        // XXX: This is the most time-critical part of the program.
        imu.check_retrieve(now, lon, lat).unwrap();
    } else {
        defmt::debug!("RTC: taking imu.");
        unsafe {
            imu.replace(IMU.take().unwrap());
        }
    }
}
