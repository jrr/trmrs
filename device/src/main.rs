use epd_waveshare::{epd7in5_v2::*, prelude::*};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::thread;
use std::time::Duration;

mod draw;
use trmrs_core::dimensions::Dimensions;
mod wifi;

use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{config::Config, SpiDeviceDriver, SpiDriverConfig};

const PIN_BUTTON: i32 = 2;

const SCREEN_DIMS: Dimensions = Dimensions {
    width: 800,
    height: 480,
};

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
static BUTTON_EVENT_OCCURRED: AtomicBool = AtomicBool::new(false);
static BUTTON_PRESS_TIME: AtomicI32 = AtomicI32::new(0);
static LAST_ACTIVITY_TIME: AtomicI32 = AtomicI32::new(0);

const FERRIS_PNG: &[u8] = include_bytes!("../sample_images/ferris-floyd.png");
const HEXAGONS_PNG: &[u8] = include_bytes!("../sample_images/hexagons.png");
const SPECKLE_PNG: &[u8] = include_bytes!("../sample_images/speckle.png");

#[derive(Debug, Clone)]
enum Scene {
    Text,
    Ferris,
    Hexagons,
    SpecklePng,
    RandomNoise,
}

fn draw_random_noise(buffer: &mut [u8]) {
    for byte in buffer {
        *byte = rand::random::<u8>();
    }
}

fn render_scene(scene: &Scene, buffer: &mut [u8]) -> anyhow::Result<()> {
    match scene {
        Scene::Text => {
            log::info!("Displaying text");
            #[allow(static_mut_refs)]
            let what: String = unsafe { WIFI_STATUS.clone() };
            let core_message = trmrs_core::hello_world() + "\n" + &what;
            draw::draw_text(buffer, &core_message, SCREEN_DIMS.width, SCREEN_DIMS.height);
        }
        Scene::Ferris => {
            log::info!("Displaying Ferris");
            trmrs_core::png::decode_and_center_png(buffer, FERRIS_PNG, SCREEN_DIMS)?;
        }
        Scene::Hexagons => {
            log::info!("Displaying Hexagons");
            trmrs_core::png::decode_and_center_png(buffer, HEXAGONS_PNG, SCREEN_DIMS)?;
        }
        Scene::SpecklePng => {
            log::info!("Displaying Speckle");
            trmrs_core::png::decode_and_center_png(buffer, SPECKLE_PNG, SCREEN_DIMS)?;
        }
        Scene::RandomNoise => {
            log::info!("Displaying random noise");
            draw_random_noise(buffer);
        }
    }
    Ok(())
}

static mut WIFI_STATUS: String = String::new();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Starting e-paper display test");
    log::info!("Embedded PNG size: {} bytes", FERRIS_PNG.len());

    let now = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as i32 };
    LAST_ACTIVITY_TIME.store(now, Ordering::SeqCst);

    let peripherals = Peripherals::take().unwrap();

    let wifi_ssid = option_env!("WIFI_SSID");
    let wifi_pass = option_env!("WIFI_PASS");

    unsafe {
        WIFI_STATUS = if let (Some(ssid), Some(pass)) = (&wifi_ssid, &wifi_pass) {
            if !ssid.is_empty() && !pass.is_empty() {
                log::info!("WiFi: credentials found, initializing.");
                log::info!("Wifi: connecting to SSID {wifi_ssid:?}");
                let mut wifi = wifi::setup_wifi(peripherals.modem).await?;
                let ip = wifi::connect_wifi(&mut wifi, ssid, pass).await?;
                ip
            } else {
                log::info!("Skipping WiFi - credentials are empty");
                "(empty wifi credentials)".to_string()
            }
        } else {
            log::info!("Skipping WiFi - credentials not provided");
            "(no wifi credentials)".to_string()
        };
    }

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

    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio7;
    let mosi = peripherals.pins.gpio8;

    let rst = PinDriver::output(peripherals.pins.gpio10)?;
    let dc = PinDriver::output(peripherals.pins.gpio5)?;
    let busy = PinDriver::input(peripherals.pins.gpio4)?;
    let _cs = PinDriver::output(peripherals.pins.gpio6)?;

    log::info!("SPI pins initialized");

    log::info!("Configuring SPI");
    let config = Config::new().baudrate(4_000_000.into());

    log::info!("Creating SPI driver");
    let mut spi_driver = SpiDeviceDriver::new_single(
        spi,
        sclk,
        mosi,
        Option::<Gpio0>::None,
        Option::<AnyIOPin>::None,
        &SpiDriverConfig::new(),
        &config,
    )?;

    log::info!("SPI driver created successfully");

    let mut delay = Delay::new_default();

    let mut epd = Epd7in5::new(&mut spi_driver, busy, dc, rst, &mut delay, Option::None)?;
    log::info!("E-paper display initialized");

    thread::sleep(Duration::from_millis(100));
    epd.clear_frame(&mut spi_driver, &mut delay)?;
    thread::sleep(Duration::from_millis(100));

    let mut buffer = vec![0u8; (SCREEN_DIMS.area() / 8) as usize];

    // was having trouble with this:
    // let mut display = Display7in5::default();

    let scenes = [
        Scene::Text,
        Scene::Ferris,
        Scene::Hexagons,
        Scene::SpecklePng,
        Scene::RandomNoise,
    ];
    let mut current_scene_index = 0;

    render_scene(&scenes[current_scene_index], &mut buffer)?;
    epd.update_and_display_frame(&mut spi_driver, &buffer, &mut delay)?;

    log::info!("Starting main loop");
    let inactivity_timeout = 60_000;

    log::info!(
        "Stack high-water: {} bytes",
        unsafe { esp_idf_sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()) } * 4
    );

    loop {
        thread::sleep(Duration::from_millis(200));

        let current_time = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as i32 };
        let last_activity = LAST_ACTIVITY_TIME.load(Ordering::SeqCst);
        let idle_time = current_time.wrapping_sub(last_activity);

        if idle_time > inactivity_timeout {
            log::info!("Shutting down due to inactivity ({}s)", idle_time / 1_000);

            log::info!("Putting display to sleep");
            epd.sleep(&mut spi_driver, &mut delay)?;

            log::info!("Going to deep sleep now");
            unsafe {
                esp_idf_sys::esp_deep_sleep_start();
            }
        }

        if BUTTON_EVENT_OCCURRED.load(Ordering::SeqCst) {
            BUTTON_EVENT_OCCURRED.store(false, Ordering::SeqCst);

            LAST_ACTIVITY_TIME.store(current_time, Ordering::SeqCst);

            let level = unsafe { esp_idf_sys::gpio_get_level(PIN_BUTTON) };

            button_pin.enable_interrupt()?;

            // With pull-up resistor: 0 = pressed, 1 = released
            if level == 0 {
                log::info!("Button press");
            } else {
                let press_time = BUTTON_PRESS_TIME.load(Ordering::SeqCst);
                let now = unsafe { (esp_idf_sys::esp_timer_get_time() / 1000) as i32 };
                let duration = now - press_time;

                log::info!("Button release ({duration}ms)");

                current_scene_index = (current_scene_index + 1) % scenes.len();

                let current_scene = &scenes[current_scene_index];
                render_scene(current_scene, &mut buffer)?;

                epd.update_and_display_frame(&mut spi_driver, &buffer, &mut delay)?;

                log::info!("End loop, free heap: {} bytes", unsafe {
                    esp_idf_sys::esp_get_free_heap_size()
                });
            }
        }
    }
}
