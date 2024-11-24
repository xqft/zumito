use core::cell::RefCell;

use critical_section::Mutex;
use esp_hal::{
    gpio::{AnyPin, Event, Input, Io, Pull},
    macros::handler,
    InterruptConfigurable,
};

// echo input mutex to access in handler later
static ECHO: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

pub fn setup_echo_interrupt(echo_gpio: AnyPin, io: &mut Io) {
    // set an interrupt handler (will be called on any interrupt)
    io.set_interrupt_handler(echo_interrupt_handler);
    // set echo pin
    let mut echo = Input::new(echo_gpio, Pull::Up);

    // critical section because we're writing on shared mutex
    critical_section::with(|cs| {
        // listen on interrupt of any edge
        echo.listen(Event::AnyEdge);
        // write input to mutex
        ECHO.borrow_ref_mut(cs).replace(echo);
    })
}

#[embassy_executor::task]
async fn control() {
    // trigger and manage detection events
}

#[handler]
fn echo_interrupt_handler() {
    critical_section::with(|cs| {
        if ECHO.borrow_ref_mut(cs).as_mut().unwrap().is_interrupt_set() {
            ECHO
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt();
        }
    })
}
