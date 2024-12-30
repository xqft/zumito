#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{AnyPin, Io, Level, Output},
    prelude::*,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_wifi::wifi::WifiStaDevice;
use log::info;
use zumito::{
    control,
    motor::{self},
    net::{self, udp},
    ultrasonic::{self},
};

#[embassy_executor::task]
async fn print_distances() {
    loop {
        let (d0, d1) = join(ultrasonic::DISTANCE0.wait(), ultrasonic::DISTANCE1.wait()).await;
        info!("distances: {} mm, {} mm", d0, d1);
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn blink_led(pin: AnyPin) {
    let mut led = Output::new(pin, Level::High);
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(200)).await;
    }
}

#[main]
async fn main(spawner: Spawner) -> ! {
    // init logger
    esp_println::logger::init_logger_from_env();

    // alloc heap (mostly used for wifi)
    esp_alloc::heap_allocator!(72 * 1024);

    // init hal, take peripherals
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // init embasssy
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // init wifi
    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let esp_wifi_controller = esp_wifi::init(
        timg1.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();
    let esp_wifi_controller_ref = net::wifi::set_esp_wifi_controller(esp_wifi_controller);
    let (wifi_interface, wifi_controller) =
        esp_wifi::wifi::new_with_mode(esp_wifi_controller_ref, peripherals.WIFI, WifiStaDevice)
            .unwrap();
    net::wifi::register(&spawner, wifi_controller, wifi_interface).await;

    // init udp receiver
    udp::register(&spawner, net::wifi::get_stack().await);

    // motor pwm
    motor::register(
        &spawner,
        [peripherals.GPIO25.into(), peripherals.GPIO26.into()],
        [
            peripherals.GPIO27.into(),
            peripherals.GPIO14.into(),
            peripherals.GPIO12.into(),
            peripherals.GPIO13.into(),
        ],
        peripherals.MCPWM0,
    )
    .expect("failed to register motors");

    let mut io = Io::new(peripherals.IO_MUX);

    // TOOD: better error handling (use logs? defmt or smth like that)
    ultrasonic::register(
        &spawner,
        &mut io,
        [peripherals.GPIO34.into(), peripherals.GPIO35.into()],
        [peripherals.GPIO32.into(), peripherals.GPIO33.into()],
    )
    .expect("failed to register ultrasonic sensors");

    control::manual::spawn(&spawner);

    spawner.spawn(print_distances()).unwrap();
    spawner.spawn(blink_led(peripherals.GPIO2.into())).unwrap();

    // 34: ECHO1
    // 35: ECHO2
    // 32: TRIG1
    // 33: TRIG2

    // 25: MCPWM1
    // 26: MCPWM2
    // 27: MCDIR11
    // 14: MCDIR12
    // 12: MCDIR21
    // 13: MCDIR22

    // 4: IR1
    // 16: IR2

    loop {
        Timer::at(Instant::MAX).await;
    }
}
