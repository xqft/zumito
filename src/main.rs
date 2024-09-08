#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Io, Level, Output},
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    timer::timg::TimerGroup,
};
use zumito::motor::DoubleMotorConfig;

#[main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // setup timer0
    let clocks = ClockControl::max(system.clock_control).freeze();
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    //embassy::init(&clocks, timer_group0.timer0);

    // setup delay
    let delay = Delay::new(&clocks);

    // motor pwm
    let mut motor_config =
        DoubleMotorConfig::take(io.pins.gpio32, io.pins.gpio33, peripherals.MCPWM0, &clocks);

    let mut led = Output::new(io.pins.gpio2, Level::High);

    let mut duty = 0.;
    loop {
        motor_config.set_duty_cycle_a(duty);
        led.toggle();
        delay.delay(1000.millis());
        duty = (duty + 0.15) % 1.0;
    }
}
