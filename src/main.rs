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
    timer::timg::TimerGroup,
};
use log::info;
use zumito::{
    motor::{self, Direction},
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
async fn update_motors() {
    let mut duty = 0;
    loop {
        duty += motor::PWM_PERIOD / 8;
        motor::MOTOR_1.signal((duty, Direction::Forward));
        info!("set motor A to duty {}/{}", duty, motor::PWM_PERIOD);
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

    spawner.spawn(print_distances()).unwrap();
    spawner.spawn(update_motors()).unwrap();
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

    // 0: IR1
    // 4: IR2

    loop {
        Timer::at(Instant::MAX).await;
    }
}
