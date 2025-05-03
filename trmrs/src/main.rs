use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyleBuilder, Rectangle},
};
use epd_waveshare::{color::Color, epd7in5_v2::*, graphics::DisplayRotation, prelude::*};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::io::Cursor;
use std::thread;
use std::time::Duration;

const FERRIS_FLOYD_PNG: &[u8] = include_bytes!("../ferris-floyd.png");

// ESP-IDF imports
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{config::Config, SpiDeviceDriver, SpiDriverConfig};

const PIN_BUTTON: i32 = 2; // Default button pin on TRMNL board

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
static BUTTON_EVENT_OCCURRED: AtomicBool = AtomicBool::new(false);
static BUTTON_PRESS_TIME: AtomicI32 = AtomicI32::new(0);

fn draw_random_noise(buffer: &mut [u8]) {
    for byte in buffer {
        *byte = rand::random::<u8>();
    }
}

fn decode_and_center_png(buffer: &mut [u8]) -> anyhow::Result<()> {
    const SCREEN_WIDTH: u32 = 800;
    const SCREEN_HEIGHT: u32 = 480;

    buffer.fill(0x00);

    let decoder = png::Decoder::new(Cursor::new(FERRIS_FLOYD_PNG));
    let mut reader = decoder.read_info()?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;
    log::info!("PNG info: {:?}", info);

    let x_offset = (SCREEN_WIDTH - width) / 2;
    let y_offset = (SCREEN_HEIGHT - height) / 2;

    log::info!("Centering PNG at offset ({}, {})", x_offset, y_offset);

    let mut img_data = vec![0; reader.output_buffer_size()];

    let frame = reader.next_frame(&mut img_data)?;

    let bytes_per_row = (width + 7) / 8;

    for y in 0..frame.height {
        for byte_x in 0..bytes_per_row {
            let src_idx = y as usize * bytes_per_row as usize + byte_x as usize;
            if src_idx >= img_data.len() {
                continue;
            }
            let src_byte = img_data[src_idx];

            // In 1-bit grayscale, 0 = black, 1 = white
            // For e-ink display: 0 = white, 1 = black, so we need to invert
            let mut display_byte = !src_byte; // Invert the bits

            // Handle the right edge (last byte in row) where we might need to mask out padding bits
            if byte_x == bytes_per_row - 1 && width % 8 != 0 {
                let padding_bits = 8 - (width % 8);
                let mask = 0xFF << padding_bits;
                display_byte &= mask;
            }

            let dest_y = (y_offset + y) as usize;
            let dest_x_byte = ((x_offset / 8) + byte_x) as usize;
            let dest_idx = dest_y * (SCREEN_WIDTH / 8) as usize + dest_x_byte;

            if dest_idx < buffer.len() {
                buffer[dest_idx] = display_byte;
            }
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Starting e-paper display test");
    log::info!("Embedded PNG size: {} bytes", FERRIS_FLOYD_PNG.len());
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

    // Draw random noise initially
    draw_random_noise(&mut buffer);

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

                // Toggle between random noise and Ferris image
                static SHOW_FERRIS: AtomicBool = AtomicBool::new(false);
                let show_ferris = !SHOW_FERRIS.load(Ordering::SeqCst);
                SHOW_FERRIS.store(show_ferris, Ordering::SeqCst);

                if show_ferris {
                    log::info!("Displaying Ferris image");
                    decode_and_center_png(&mut buffer)?;
                } else {
                    log::info!("Displaying random noise");
                    draw_random_noise(&mut buffer);
                }

                epd.update_and_display_frame(&mut spi_driver, &buffer, &mut delay)?;
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
