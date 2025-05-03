use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyleBuilder, Rectangle},
};
use epd_waveshare::{color::Color, epd7in5_v2::*, graphics::DisplayRotation, prelude::*};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use std::time::Duration;

// ESP-IDF imports
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{config::Config, SpiDeviceDriver, SpiDriverConfig};

const PIN_BUTTON: i32 = 2; // Default button pin on TRMNL board

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
static BUTTON_EVENT_OCCURRED: AtomicBool = AtomicBool::new(false);
static BUTTON_PRESS_TIME: AtomicI32 = AtomicI32::new(0);

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Starting e-paper display test");
    thread::sleep(Duration::from_millis(500));

    // Step 1: Get peripherals
    log::info!("Initializing peripherals");
    let peripherals = Peripherals::take().unwrap();
    log::info!("Peripherals initialized");

    // Step 2: Initialize SPI pins for display
    log::info!("Initializing SPI pins");
    log::info!("Initializing button on GPIO{}", PIN_BUTTON);

    let mut button_pin = PinDriver::input(peripherals.pins.gpio2)?;
    button_pin.set_pull(Pull::Up)?;

    button_pin.set_interrupt_type(InterruptType::AnyEdge)?;

    let button_interrupt = move || {
        let level = unsafe { esp_idf_sys::gpio_get_level(PIN_BUTTON) };

        let now = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as i32 };

        if level == 0 {
            BUTTON_PRESS_TIME.store(now, Ordering::SeqCst);
        }

        BUTTON_EVENT_OCCURRED.store(true, Ordering::SeqCst);
    };

    unsafe {
        button_pin.subscribe(button_interrupt)?;
    }

    button_pin.enable_interrupt()?;

    // Extract pins for SPI and display control
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio7;
    let mosi = peripherals.pins.gpio8;

    // Create pin drivers for display control
    let rst = PinDriver::output(peripherals.pins.gpio10)?;
    let dc = PinDriver::output(peripherals.pins.gpio5)?;
    let busy = PinDriver::input(peripherals.pins.gpio4)?;
    let cs = PinDriver::output(peripherals.pins.gpio6)?;

    log::info!("SPI pins initialized");

    // Configure SPI
    log::info!("Configuring SPI");
    let config = Config::new().baudrate(4_000_000.into()); // 4MHz to be safer

    // Create SPI driver
    log::info!("Creating SPI driver");
    let mut spi_driver = SpiDeviceDriver::new_single(
        spi,
        sclk,
        mosi,
        Option::<Gpio0>::None,    // No MISO needed
        Option::<AnyIOPin>::None, // CS is handled manually
        &SpiDriverConfig::new(),
        &config,
    )?;

    log::info!("SPI driver created successfully");

    let mut delay = Ets;

    let mut epd = Epd7in5::new(&mut spi_driver, busy, dc, rst, &mut delay, Option::None)?;
    log::info!("E-paper display initialized");

    // Create a buffer for the display (800x480 bits)
    let mut buffer = vec![0u8; (800 * 480) / 8];

    for byte in &mut buffer {
        *byte = rand::random::<u8>();
    }

    epd.update_and_display_frame(&mut spi_driver, &buffer, &mut delay)?;

    // was having trouble with this:
    // let mut display = Display7in5::default();

    log::info!("Starting main loop");
    loop {
        thread::sleep(Duration::from_millis(200));

        // Check if a button event occurred
        if BUTTON_EVENT_OCCURRED.load(Ordering::SeqCst) {
            BUTTON_EVENT_OCCURRED.store(false, Ordering::SeqCst);

            let level = unsafe { esp_idf_sys::gpio_get_level(PIN_BUTTON) };

            button_pin.enable_interrupt()?;

            // With pull-up resistor: 0 = pressed, 1 = released
            if level == 0 {
                log::info!("Button press");
            } else {
                // Button released - calculate duration
                let press_time = BUTTON_PRESS_TIME.load(Ordering::SeqCst);
                let now = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as i32 }; // Convert microseconds to milliseconds
                let duration = now - press_time; // Safe even with wrap-around due to two's complement

                log::info!("Button release ({}ms)", duration);
            }
        }
    }
}
