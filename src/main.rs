#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use core::cell::RefCell;

use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Io, Level, Output},
    prelude::*,
    timer::timg::TimerGroup,
};
use zumito::{
    motor::DoubleMotorConfig,
    sensor::{self},
};

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
    io.set_interrupt_handler(handler);

    let ultrasonic_sensor_future =
        sensor::new(peripherals.GPIO25.into(), peripherals.GPIO26.into());

    let isr = || {
        ultrasonic_sensor_future.echo_interrupt_handler();
    };

    #[handler]
    fn handler() {
        isr();
    }

    let mut duty = 0.;
    loop {
        motor_config.set_duty_cycle_a(duty);
        led.toggle();
        Timer::after(Duration::from_secs(1)).await;
        duty = (duty + 0.15) % 1.0;
    }
}
