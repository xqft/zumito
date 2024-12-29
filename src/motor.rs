use embassy_executor::{SpawnError, Spawner};
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{
    gpio::{AnyPin, Level, Output},
    mcpwm::{
        operator::{PwmPin, PwmPinConfig},
        timer::PwmWorkingMode,
        FrequencyError, McPwm, PeripheralClockConfig,
    },
    peripherals::MCPWM0,
    prelude::*,
};

/// Uses MCPWM0, operator 0 and pin A.
type PWMA<'d> = PwmPin<'d, MCPWM0, 0, true>;
/// Uses MCPWM0, operator 0 and pin B.
type PWMB<'d> = PwmPin<'d, MCPWM0, 0, false>;

const CLOCK_FREQ_MHZ: u32 = 40;
const PWM_FREQ_KHZ: u32 = 10;

pub const PWM_PERIOD: u8 = u8::MAX;

pub static MOTOR_1: Signal<CriticalSectionRawMutex, (u8, Direction)> = Signal::new();
pub static MOTOR_2: Signal<CriticalSectionRawMutex, (u8, Direction)> = Signal::new();

pub fn register(
    spawner: &Spawner,
    pwm_pins: [AnyPin; 2],
    dir_pins: [AnyPin; 4],
    mcpwm_peripheral: MCPWM0,
) -> Result<(), MotorError> {
    let peripheral_clock = PeripheralClockConfig::with_frequency(CLOCK_FREQ_MHZ.MHz())?;
    let mut mcpwm = McPwm::new(mcpwm_peripheral, peripheral_clock);

    let [pwm_pin1, pwm_pin2] = pwm_pins;

    mcpwm.operator0.set_timer(&mcpwm.timer0);
    let (pwm_1, pwm_2) = mcpwm.operator0.with_pins(
        pwm_pin1,
        PwmPinConfig::UP_ACTIVE_HIGH,
        pwm_pin2,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    let timer_clock_cfg = peripheral_clock.timer_clock_with_frequency(
        PWM_PERIOD.into(),
        PwmWorkingMode::Increase,
        PWM_FREQ_KHZ.kHz(),
    )?;
    mcpwm.timer0.start(timer_clock_cfg);

    spawner.spawn(handle_motors(pwm_1, pwm_2, dir_pins))?;

    Ok(())
}

#[embassy_executor::task]
async fn handle_motors(mut pwm_a: PWMA<'static>, mut pwm_b: PWMB<'static>, dir_pins: [AnyPin; 4]) {
    let [mut dir_pin11, mut dir_pin12, mut dir_pin21, mut dir_pin22] =
        dir_pins.map(|pin| Output::new(pin, Level::Low));

    loop {
        match select(MOTOR_1.wait(), MOTOR_2.wait()).await {
            Either::First((duty, dir)) => {
                let levels: (Level, Level) = dir.into();
                dir_pin11.set_level(levels.0);
                dir_pin12.set_level(levels.1);
                pwm_a.set_timestamp(duty.into())
            }
            Either::Second((duty, dir)) => {
                let levels: (Level, Level) = dir.into();
                dir_pin21.set_level(levels.0);
                dir_pin22.set_level(levels.1);
                pwm_b.set_timestamp(duty.into())
            }
        }
    }
}

pub enum Direction {
    Forward,
    Reverse,
}

impl Into<(Level, Level)> for Direction {
    fn into(self) -> (Level, Level) {
        match self {
            Self::Forward => (Level::Low, Level::High),
            Self::Reverse => (Level::High, Level::Low),
        }
    }
}

// TODO: defmt
#[derive(Copy, Clone, Debug)]
pub enum MotorError {
    Spawn(SpawnError),
    Frequency(FrequencyError),
}

impl From<SpawnError> for MotorError {
    fn from(value: SpawnError) -> Self {
        MotorError::Spawn(value)
    }
}
impl From<FrequencyError> for MotorError {
    fn from(value: FrequencyError) -> Self {
        MotorError::Frequency(value)
    }
}
