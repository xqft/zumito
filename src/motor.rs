use embassy_executor::{SpawnError, Spawner};
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::{
    gpio::AnyPin,
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

pub static DUTY_A: Signal<CriticalSectionRawMutex, u8> = Signal::new();
pub static DUTY_B: Signal<CriticalSectionRawMutex, u8> = Signal::new();

pub fn register(
    spawner: &Spawner,
    pin_a: AnyPin,
    pin_b: AnyPin,
    mcpwm_peripheral: MCPWM0,
) -> Result<(), MotorError> {
    let peripheral_clock = PeripheralClockConfig::with_frequency(CLOCK_FREQ_MHZ.MHz())?;
    let mut mcpwm = McPwm::new(mcpwm_peripheral, peripheral_clock);

    mcpwm.operator0.set_timer(&mcpwm.timer0);
    let (pwm_a, pwm_b) = mcpwm.operator0.with_pins(
        pin_a,
        PwmPinConfig::UP_ACTIVE_HIGH,
        pin_b,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    let timer_clock_cfg = peripheral_clock.timer_clock_with_frequency(
        PWM_PERIOD.into(),
        PwmWorkingMode::Increase,
        PWM_FREQ_KHZ.kHz(),
    )?;
    mcpwm.timer0.start(timer_clock_cfg);

    spawner.spawn(handle_motors(pwm_a, pwm_b))?;

    Ok(())
}

#[embassy_executor::task]
async fn handle_motors(mut pwm_a: PWMA<'static>, mut pwm_b: PWMB<'static>) {
    loop {
        match select(DUTY_A.wait(), DUTY_B.wait()).await {
            Either::First(duty) => pwm_a.set_timestamp(duty.into()),
            Either::Second(duty) => pwm_b.set_timestamp(duty.into()),
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
