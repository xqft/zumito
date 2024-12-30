use core::cell::RefCell;

use embassy_sync::{
    blocking_mutex::{raw::CriticalSectionRawMutex, CriticalSectionMutex},
    signal::Signal,
};
use esp_hal::{
    gpio::{AnyPin, Event, Input, Io, Pull},
    macros::handler,
    InterruptConfigurable,
};
use portable_atomic::{AtomicU64, Ordering::Relaxed};

pub type EchoSignal = Signal<CriticalSectionRawMutex, u64>;

// Mutex cells containing the echo pin inputs
static ECHO0: CriticalSectionMutex<RefCell<Option<Input>>> =
    CriticalSectionMutex::new(RefCell::new(None));
static ECHO1: CriticalSectionMutex<RefCell<Option<Input>>> =
    CriticalSectionMutex::new(RefCell::new(None));

// Signals for advertising new flight time measure
pub static ECHO_SIGNAL0: EchoSignal = Signal::new();
pub static ECHO_SIGNAL1: EchoSignal = Signal::new();

pub fn register(io: &mut Io, pins: [AnyPin; 2]) {
    let [input0, input1] = pins.map(|pin| {
        let mut input = Input::new(pin, Pull::Down);
        input.listen(Event::AnyEdge);
        input
    });

    critical_section::with(|cs| {
        ECHO0.borrow(cs).replace(Some(input0));
        ECHO1.borrow(cs).replace(Some(input1));
    });

    io.set_interrupt_handler(isr);
}

/// The actual routine to execute during a IO interruption.
fn handle_echo<'d>(input: &mut Input<'d>, signal: &EchoSignal) {
    static INITIAL_TIME: AtomicU64 = AtomicU64::new(0);

    if !input.is_interrupt_set() {
        return;
    }

    let current_time = esp_hal::time::now().ticks();

    if input.is_high() {
        INITIAL_TIME.store(current_time, Relaxed);
    } else {
        // in microseconds:
        let flight_time = current_time - INITIAL_TIME.load(Relaxed);

        // send signal
        signal.signal(flight_time);
    }
    input.clear_interrupt();
}

#[handler]
fn isr() {
    critical_section::with(|cs| {
        handle_echo(
            ECHO0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .expect("echo input 0 not initialized"),
            &ECHO_SIGNAL0,
        );

        handle_echo(
            ECHO1
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .expect("echo input 1 not initialized"),
            &ECHO_SIGNAL1,
        );
    });
}
