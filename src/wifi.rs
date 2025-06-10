use anyhow::Result;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
    wifi::{AsyncWifi, EspWifi},
};

pub async fn setup_wifi(modem: esp_idf_hal::modem::Modem) -> Result<AsyncWifi<EspWifi<'static>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi = AsyncWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
        timer_service,
    )?;

    Ok(wifi)
}

pub async fn connect_wifi(
    wifi: &mut AsyncWifi<EspWifi<'static>>,
    ssid: &str,
    password: &str,
) -> Result<String> {
    let wifi_configuration = Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: password.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    log::info!("Connecting to WiFi SSID: {ssid}");

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start().await?;

    log::info!("WiFi started, attempting connection...");
    wifi.connect().await?;

    log::info!("WiFi connected, waiting for network interface...");
    wifi.wait_netif_up().await?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("WiFi connected! IP: {}", ip_info.ip);

    Ok(ip_info.ip.to_string())
}
