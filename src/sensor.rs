use core::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_sync::waitqueue::AtomicWaker;
use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, Event, Input, Io, Level, Output, Pull},
    macros::handler,
    InterruptConfigurable,
};
use esp_println::println;
use portable_atomic::{AtomicU64, Ordering::Relaxed};

const SOUND_SPEED: u64 = 343000; // in mm/s

// mutexes to borrow access to io
static ECHO: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static TRIG: Mutex<RefCell<Option<Output>>> = Mutex::new(RefCell::new(None));

// distance measure future, will be written and awoken by the echo ISR
static DISTANCE_MEASURE: DistanceMeasureFlag = DistanceMeasureFlag {
    waker: AtomicWaker::new(),
    flight_time: AtomicU64::new(0),
};

#[embassy_executor::task]
async fn control() {
    loop {
        let distance = DistanceMeasureFuture.await;
        println!("Measured distance: {} mm", distance);
    }
}

pub fn setup(spawner: &Spawner, echo_gpio: AnyPin, trig_pin: AnyPin, io: &mut Io) {
    // set an interrupt handler (will be called on any interrupt)
    io.set_interrupt_handler(echo_interrupt_handler);
    // define io
    let echo = Input::new(echo_gpio, Pull::Up);
    let trig = Output::new(trig_pin, Level::Low);

    // critical section because we're writing on shared mutex
    critical_section::with(|cs| {
        // write echo to mutex
        ECHO.borrow_ref_mut(cs).replace(echo);
        // write trig to mutex
        TRIG.borrow_ref_mut(cs).replace(trig);
    });

    spawner
        .spawn(control())
        .expect("failed to spawn ultrasonic sensor control task");
}

/// Triggers ultrasonic sensor.
///
/// # Panics
///
/// Panics if io was not set (didn't call setup() before)
fn trigger() {
    let delay = Delay::new();
    // set high
    critical_section::with(|cs| {
        let mut trig = TRIG.borrow_ref_mut(cs);
        trig.as_mut()
            .expect("ultrasonic sensor trigger pin not set")
            .set_high();
    });

    delay.delay_micros(10);

    critical_section::with(|cs| {
        // set low
        let mut trig = TRIG.borrow_ref_mut(cs);
        trig.as_mut()
            .expect("ultrasonic sensor trigger pin not set")
            .set_low();

        // enable echo interruption
        let mut echo = ECHO.borrow_ref_mut(cs);
        echo.as_mut()
            .expect("ultrasonic sensor echo pin not set")
            .listen(Event::AnyEdge);
    });
}

/// Represents the event of a time measure in the echo pin.
///
/// Will be set up by echo pin interruption and will wake up sensor control task.
struct DistanceMeasureFlag {
    waker: AtomicWaker,
    flight_time: AtomicU64,
}

/// Future for a distance measurement made by the sensor.
struct DistanceMeasureFuture;
impl Future for DistanceMeasureFuture {
    type Output = u64;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let flight_time = DISTANCE_MEASURE.flight_time.swap(0, Relaxed);
        if flight_time != 0 {
            // in millimeters:
            let distance = flight_time * SOUND_SPEED / 1_000_000 / 2;
            Poll::Ready(distance)
        } else {
            trigger();
            DISTANCE_MEASURE.waker.register(cx.waker());
            Poll::Pending
        }
    }
}

/// Echo pin interruption handler.
///
/// Will measure flight time and wake up the [DistanceMeasureFuture].
#[handler]
fn echo_interrupt_handler() {
    static INITIAL_TIME: AtomicU64 = AtomicU64::new(0);

    critical_section::with(|cs| {
        let mut echo_option = ECHO.borrow_ref_mut(cs);
        let echo = echo_option
            .as_mut()
            .expect("ultrasonic sensor echo pin not set");

        if !echo.is_interrupt_set() {
            return;
        }

        let current_time = esp_hal::time::now().ticks();

        if echo.is_high() {
            INITIAL_TIME.store(current_time, Relaxed);
        } else {
            echo.unlisten();
            // in microseconds:
            let flight_time = current_time - INITIAL_TIME.load(Relaxed);

            // store flight time and wake future
            DISTANCE_MEASURE.flight_time.store(flight_time, Relaxed);
            DISTANCE_MEASURE.waker.wake();
        }
        echo.clear_interrupt();
    });
}
