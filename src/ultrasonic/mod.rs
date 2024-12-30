use embassy_executor::{SpawnError, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{with_timeout, Duration, Timer};
use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, Io, Level, Output},
};

use echo_handler::EchoSignal;
use log::warn;

mod echo_handler;

const MAX_DISTANCE: u64 = 4; // in meters
const MEASURE_RATE: u64 = 10; // measures per second, or Hz.

pub static DISTANCE0: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static DISTANCE1: Signal<CriticalSectionRawMutex, u64> = Signal::new();

/// Register two (HC-SR04 or HY-SRF05) ultrasonic sensors.
pub fn register(
    spawner: &Spawner,
    io: &mut Io,
    echo_pins: [AnyPin; 2],
    trig_pins: [AnyPin; 2],
) -> Result<(), SpawnError> {
    echo_handler::register(io, echo_pins);
    spawner.spawn(handle_sensors(trig_pins))
}

/// Ultrasonic sensor handling task, in charge of triggering the sensors and signalling new measures.
#[embassy_executor::task]
async fn handle_sensors(trig_pins: [AnyPin; 2]) {
    let [trig0, trig1] = trig_pins.map(|pin| Output::new(pin, Level::Low));
    let mut ultrasonic0 = Ultrasonic {
        trig: trig0,
        echo_signal: &echo_handler::ECHO_SIGNAL0,
    };
    let mut ultrasonic1 = Ultrasonic {
        trig: trig1,
        echo_signal: &echo_handler::ECHO_SIGNAL1,
    };

    const TIMEOUT: Duration = Duration::from_millis(MAX_DISTANCE * 1000 / 343);
    const MEASURE_DELAY: Duration = Duration::from_millis(1000 / MEASURE_RATE);
    loop {
        // sequential measure so the pulse of a sensor doesn't interfere with the other one
        let distance0 = with_timeout(TIMEOUT, ultrasonic0.measure())
            .await
            .unwrap_or_default();
        Timer::after(MEASURE_DELAY / 2).await;
        let distance1 = with_timeout(TIMEOUT, ultrasonic1.measure())
            .await
            .unwrap_or_default();
        Timer::after(MEASURE_DELAY / 2).await;

        // if a sensor doesn't receive the pulse back, skip the measure and wait until the echo pin times out
        if distance0 == 0 || distance1 == 0 {
            warn!("ultrasonic measure failed, skipping");
            // apparently the echo pins have a big timeout, I found that one second works well
            Timer::after_secs(1).await;
            continue;
        }

        DISTANCE0.signal(distance0);
        DISTANCE1.signal(distance1);
    }
}

/// Ultrasonic sensor implementation based on the HC-SR04 or HY-SRF05.
struct Ultrasonic<'d> {
    trig: Output<'d>,
    echo_signal: &'d EchoSignal,
}

impl<'d> Ultrasonic<'d> {
    /// Triggers the sensor, awaits for an echo signal and calculates the distance.
    async fn measure(&mut self) -> u64 {
        self.trigger();
        let flight_time = self.echo_signal.wait().await;
        Self::distance(flight_time)
    }

    /// Triggers ultrasonic sensor. Blocks execution by 10 us.
    fn trigger(&mut self) {
        let delay = Delay::new();
        self.trig.set_high();

        delay.delay_micros(10);

        self.trig.set_low();
    }

    /// Calculates distance based on flight time.
    ///
    /// Flight time should be in microseconds. The returned distance is in mm.
    fn distance(flight_time: u64) -> u64 {
        const SOUND_SPEED: u64 = 343000; // mm/s
        flight_time * SOUND_SPEED / 1_000_000 / 2 // mm
    }
}
