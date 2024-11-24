#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Io, Level, Output},
    prelude::*,
    timer::timg::TimerGroup,
};
use zumito::{motor::DoubleMotorConfig, sensor};

#[main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // setup timer0
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // motor pwm
    let mut motor_config = DoubleMotorConfig::take(
        peripherals.GPIO32.into(),
        peripherals.GPIO33.into(),
        peripherals.MCPWM0,
    );

    let mut led = Output::new(peripherals.GPIO2, Level::High);

    let mut io = Io::new(peripherals.IO_MUX);
    sensor::setup(
        &spawner,
        peripherals.GPIO25.into(),
        peripherals.GPIO26.into(),
        &mut io,
    );

    let mut duty = 0.;
    loop {
        motor_config.set_duty_cycle_a(duty);
        led.toggle();
        Timer::after(Duration::from_secs(1)).await;
        duty = (duty + 0.15) % 1.0;
    }
}
