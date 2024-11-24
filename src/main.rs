#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output},
    prelude::*,
    timer::timg::TimerGroup,
};
use zumito::motor::DoubleMotorConfig;

#[main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // setup timer0
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // setup delay
    let delay = Delay::new();

    // motor pwm
    let mut motor_config = DoubleMotorConfig::take(
        peripherals.GPIO32.into(),
        peripherals.GPIO33.into(),
        peripherals.MCPWM0,
    );

    let mut led = Output::new(peripherals.GPIO2, Level::High);

    let mut duty = 0.;
    loop {
        motor_config.set_duty_cycle_a(duty);
        led.toggle();
        delay.delay(1000.millis());
        duty = (duty + 0.15) % 1.0;
    }
}
