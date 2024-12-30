use embassy_executor::Spawner;
use log::debug;

use crate::{
    motor::{Direction, MOTOR_1, MOTOR_2},
    net::udp::RX_MSG,
};

pub fn spawn(spawner: &Spawner) {
    spawner.spawn(control()).unwrap();
}

#[embassy_executor::task]
async fn control() {
    loop {
        let (msg_len, msg) = RX_MSG.wait().await;
        if msg_len != 3 {
            continue;
        }

        MOTOR_1.signal((msg[1], Direction::Forward));
        MOTOR_2.signal((msg[2], Direction::Forward));

        debug!("set M1: {}, M2: {}", msg[1], msg[2]);
    }
}
