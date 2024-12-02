use embassy_executor::{SpawnError, Spawner};
use embassy_futures::join::join;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{with_timeout, Duration};
use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, Io, Level, Output},
};

use echo_handler::EchoSignal;

mod echo_handler;

const TIMEOUT: Duration = Duration::from_millis(200);

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

    loop {
        let sensor_future = join(ultrasonic0.measure(), ultrasonic1.measure());
        let (distance0, distance1) = with_timeout(TIMEOUT, sensor_future)
            .await
            .unwrap_or_default();
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
