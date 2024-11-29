use core::{
    borrow::BorrowMut,
    cell::OnceCell,
    future::Future,
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};

use embassy_sync::{
    blocking_mutex::{raw::CriticalSectionRawMutex, Mutex},
    signal::Signal,
};
use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, Event, Input, Level, Output, Pull},
};
use portable_atomic::{AtomicU64, Ordering::Relaxed};

/// A cell type to store an ultrasonic sensor instance.
type UltrasonicCell<'d> = Mutex<CriticalSectionRawMutex, OnceCell<Ultrasonic<'d>>>;
/// A signal type to send flight time and distance measurements.
type UltrasonicSignal = Signal<CriticalSectionRawMutex, u64>;

static ULTRASONICS: [UltrasonicCell; 2] =
    [Mutex::new(OnceCell::new()), Mutex::new(OnceCell::new())];

static FLIGHT_TIME_SIGNALS: [UltrasonicSignal; 2] =
    [UltrasonicSignal::new(), UltrasonicSignal::new()];

pub static DISTANCE_SIGNALS: [UltrasonicSignal; 2] =
    [UltrasonicSignal::new(), UltrasonicSignal::new()];

struct Ultrasonic<'d> {
    echo: Input<'d>,
    trig: Output<'d>,
    id: usize,
}

impl<'d> Ultrasonic<'d> {
    /// Defines a new sensor.
    fn new(echo_pin: AnyPin, trig_pin: AnyPin, id: usize) -> Ultrasonic<'d> {
        let echo = Input::new(echo_pin, Pull::Up);
        let trig = Output::new(trig_pin, Level::Low);

        Self { echo, trig, id }
    }

    /// Triggers ultrasonic sensor.
    ///
    /// # Panics
    ///
    /// Panics if io was not set (didn't call setup() before)
    pub fn trigger(&mut self) {
        let delay = Delay::new();
        self.trig.set_high();

        delay.delay_micros(10);

        self.trig.set_low();

        // enable echo interruption
        self.echo.listen(Event::AnyEdge);
    }

    /// Calculates distance based on flight time.
    ///
    /// Flight time should be in microseconds. The returned distance is in mm.
    pub fn distance(flight_time: u64) -> u64 {
        const SOUND_SPEED: u64 = 343000; // mm/s
        flight_time * SOUND_SPEED / 1_000_000 / 2 // mm
    }

    /// Echo pin interruption handler. Needs to be made part of the GPIO ISR.
    ///
    /// Will measure flight time and wake up the sensor future.
    pub fn echo_interrupt_handler(&mut self) {
        static INITIAL_TIME: AtomicU64 = AtomicU64::new(0);

        if !self.echo.is_interrupt_set() {
            return;
        }

        let current_time = esp_hal::time::now().ticks();

        if self.echo.is_high() {
            INITIAL_TIME.store(current_time, Relaxed);
        } else {
            self.echo.unlisten();
            // in microseconds:
            let flight_time = current_time - INITIAL_TIME.load(Relaxed);

            // send signal
            FLIGHT_TIME_SIGNALS[Id].signal(flight_time);
        }
        self.echo.clear_interrupt();
    }
}

#[embassy_executor::task]
async fn trigger_task() {
    for ((sensor, flight_signal), distance_signal) in ULTRASONICS
        .iter()
        .zip(FLIGHT_TIME_SIGNALS.iter())
        .zip(DISTANCE_SIGNALS.iter())
    {
        let flight_time = flight_signal.wait().await;
        distance_signal.signal(Ultrasonic::distance(flight_time));
        sensor.lock(|sensor| {
            sensor
                .borrow_mut()
                .get_mut()
                .expect("ultrasonic sensor not registered")
                .trigger();
        })
    }
}

impl<'d> Future for UltrasonicSensorFuture<'d> {
    type Output = u64;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let flight_time = self.flight_time.swap(0, Relaxed);
        if flight_time != 0 {
            Poll::Ready(Ultrasonic::distance(flight_time))
        } else {
            self.waker.register(cx.waker());
            self.controller.trigger();
            Poll::Pending
        }
    }
}
