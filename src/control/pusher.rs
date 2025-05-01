use embassy_executor::Spawner;
use embassy_futures::join::join;
use log::{debug, info};
use portable_atomic::AtomicI128;

use crate::{
    motor::{Direction, MOTOR_1, MOTOR_2},
    net::udp::RX_MSG,
    ultrasonic::{DISTANCE0, DISTANCE1},
};
use core::sync::atomic::Ordering::Relaxed;

pub fn spawn(spawner: &Spawner) {
    spawner.spawn(control()).unwrap();
}

#[embassy_executor::task]
async fn control() {
    loop {
        let (d0, d1) = join(DISTANCE0.wait(), DISTANCE1.wait()).await;

        // x1^2 + y^2 = d1^2
        // x2^2 + y^2 = d2^2

        // x1^2 - x2^2 = d1^2 - d2^2

        // x1 = x - 35mm
        // x2 = x + 35mm

        // (x-35)^2 - (x+35)^2 = d1^2 - d2^2
        // x^2 - 35x + 1225 - x^2 - 35x - 1225 = d1^2 - d2^2
        // 70x = d2^2 - d1^2
        // x = (d2^2 - d1^2) / 70

        //let x = ((d0 as i128 ^ 2) - (d1 as i128 ^ 2)) / 70 as i128;

        //info!("d0: {d0}, d1: {d1}, x: {x}");

        static D: AtomicI128 = AtomicI128::new(0);
        let d = d0 as i128 - d1 as i128;

        D.store((D.load(Relaxed) + d) / 2, Relaxed);
        let d = D.load(Relaxed);
        info!("d: {d}");

        let speed = 127;
        let threshold = 150;

        if d.abs() < threshold {
            MOTOR_1.signal(Ok((speed as u8, Direction::Forward))),
            MOTOR_2.signal(Ok((speed, Direction::Forward)))));
        } else if d > threshold {
            MOTOR_1.signal(Ok((speed as u8, Direction::Forward))),
            MOTOR_2.signal(Ok((0, Direction::Forward)))));
        } else {
            MOTOR_1.signal(Ok((0, Direction::Forward))),
            MOTOR_2.signal(Ok((speed, Direction::Forward)))));
        }
    }
}
