#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{AnyPin, Io, Level, Output},
    prelude::*,
    timer::timg::TimerGroup,
};
use esp_println::println;
use zumito::{
    motor,
    ultrasonic::{self},
};

#[embassy_executor::task]
async fn print_distances() {
    loop {
        let (d0, d1) = join(ultrasonic::DISTANCE0.wait(), ultrasonic::DISTANCE1.wait()).await;
        println!("distances: {} mm, {} mm", d0, d1);
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn update_motors() {
    let mut duty = 0;
    loop {
        duty += motor::PWM_PERIOD / 8;
        motor::DUTY_A.signal(duty);
        println!("set motor A to duty {}/{}", duty, motor::PWM_PERIOD);
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn blink_led(pin: AnyPin) {
    let mut led = Output::new(pin, Level::High);

    loop {
        led.toggle();
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // setup timer0
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // motor pwm
    motor::register(
        &spawner,
        peripherals.GPIO32.into(),
        peripherals.GPIO33.into(),
        peripherals.MCPWM0,
    )
    .expect("failed to register motors");

    let mut io = Io::new(peripherals.IO_MUX);

    // TOOD: better error handling (use logs? defmt or smth like that)
    ultrasonic::register(
        &spawner,
        &mut io,
        [peripherals.GPIO25.into(), peripherals.GPIO27.into()],
        [peripherals.GPIO26.into(), peripherals.GPIO14.into()],
    )
    .expect("failed to register ultrasonic sensors");

    spawner.spawn(print_distances()).unwrap();
    spawner.spawn(update_motors()).unwrap();
    spawner.spawn(blink_led(peripherals.GPIO2.into())).unwrap();

    loop {}
}
