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

    // Initialize delay for the EPD
    let mut delay = Ets;

    // Initialize e-paper display
    log::info!("Initializing e-paper display");

    // The implementation uses generics to support different pin configurations
    let mut epd = Epd7in5::new(&mut spi_driver, busy, dc, rst, &mut delay, Option::None)?;
    log::info!("E-paper display initialized");

    // Clear display (white background)
    log::info!("Clearing display");
    epd.clear_frame(&mut spi_driver, &mut delay)?;

    thread::sleep(Duration::from_millis(1000));

    log::info!("Displaying frame");
    epd.display_frame(&mut spi_driver, &mut delay)?;

    // Create a buffer for the display (800x480 bits)
    let mut buffer = vec![0u8; (800 * 480) / 8];

    // Fill buffer with noise
    for byte in &mut buffer {
        *byte = rand::random::<u8>();
    }

    epd.update_and_display_frame(&mut spi_driver, &buffer, &mut delay)?;
    thread::sleep(Duration::from_millis(1000));

    // Create the display buffer
    let mut display = Display7in5::default();

    // // Set rotation and clear the display to white
    // display.set_rotation(DisplayRotation::Rotate0);

    // // Fill the entire display with white
    // // let fill_result = display.fill_solid(
    // //     &Rectangle::new(Point::new(0, 0), Size::new(800, 480)),
    // //     Color::White,
    // // );
    // // log::info!("Fill background result: {:?}", fill_result);

    // // Add a small black rectangle in the center
    // let rect_result = display.fill_solid(
    //     &Rectangle::new(Point::new(50, 50), Size::new(100, 100)),
    //     Color::Black,
    // );
    // log::info!("Draw rectangle result: {:?}", rect_result);

    // Update the display to show the pixel
    // log::info!("Updating frame with display buffer");
    // epd.update_frame(&mut spi_driver, display.buffer(), &mut delay)?;
    // epd.display_frame(&mut spi_driver, &mut delay)?;

    // log::info!("Display updated successfully");
    // log::info!("Putting display to sleep to save power");
    // epd.sleep(&mut spi_driver, &mut delay)?;

    // Loop with proper sleep to avoid watchdog timeouts
    let mut counter = 0;
    log::info!("Starting main loop");
    loop {
        thread::sleep(Duration::from_millis(1000));
        counter += 1;
        log::info!("Loop count: {}", counter);
    }
}
