use core::str::FromStr;

use embassy_executor::Spawner;
use embassy_net::{
    udp::{self, PacketMetadata, UdpSocket},
    DhcpConfig, Stack, StackResources,
};
use embassy_time::{Duration, Timer};
use esp_wifi::{
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiController,
};
use heapless::String;
use log::{error, info, warn};
use static_cell::StaticCell;

const HOSTNAME: &str = "zumito";

static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController<'_>> = StaticCell::new();
static WIFI_CONTROLLER: StaticCell<WifiController<'_>> = StaticCell::new();
static STACK: StaticCell<Stack<WifiDevice<'_, WifiStaDevice>>> = StaticCell::new();
static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

pub fn set_esp_wifi_controller(
    esp_wifi_controller: EspWifiController<'static>,
) -> &'static mut EspWifiController<'_> {
    ESP_WIFI_CONTROLLER.init(esp_wifi_controller)
}

pub async fn register<'a>(
    spawner: &Spawner,
    wifi_controller: WifiController<'static>,
    wifi_interface: WifiDevice<'static, WifiStaDevice>,
) {
    let mut dhcp_config = DhcpConfig::default();
    dhcp_config.hostname = Some(String::from_str(HOSTNAME).unwrap());

    let config = embassy_net::Config::dhcpv4(Default::default());

    // TODO: more secure seed
    let seed = 42;

    let wifi_controller_ref = WIFI_CONTROLLER.init(wifi_controller);
    let resources_ref = STACK_RESOURCES.init(StackResources::<3>::new());
    let stack_ref = STACK.init(Stack::new(wifi_interface, config, resources_ref, seed));

    spawner.spawn(connect(wifi_controller_ref)).unwrap();
    spawner.spawn(process(stack_ref)).unwrap();

    info!("waiting until link is up");
    loop {
        if stack_ref.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("waiting to get IP address");
    loop {
        if let Some(config) = stack_ref.config_v4() {
            info!("IP address: {}", config.address);
            info!("hostname: {HOSTNAME}");
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
async fn handle_udp() {}

/// Task for establishing and maintaining a connection
#[embassy_executor::task]
async fn connect(controller: &'static mut WifiController<'static>) {
    const SSID: &str = env!("SSID");
    const PASSWORD: &str = env!("PASSWORD");

    if SSID.is_empty() || PASSWORD.is_empty() {
        error!("WiFi SSID or password is empty. Specify them as environment variables.");
        return;
    }

    info!("start connection task");
    info!("device capabilities: {:?}", controller.capabilities());

    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                warn!("wifi connection lost");
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("starting wifi");
            controller.start_async().await.unwrap();
            info!("wifi started!");
        }
        info!("about to connect");

        match controller.connect_async().await {
            Ok(_) => info!("wifi connected!"),
            Err(e) => {
                info!("failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

/// Background task to process network events
#[embassy_executor::task]
async fn process(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
