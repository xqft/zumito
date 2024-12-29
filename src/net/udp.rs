use core::str::from_utf8;

use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::Stack;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use log::{info, warn};

const PORT: u16 = 8080;

#[embassy_executor::task]
async fn handle(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 1024];

    // empty as there will be no transmission
    let mut tx_meta = [PacketMetadata::EMPTY; 0];
    let mut tx_buffer = [0; 0];

    let mut msg_buffer = [0; 32];

    let mut udp_socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    udp_socket.bind(PORT).unwrap();

    loop {
        let (_, from_addr) = udp_socket.recv_from(&mut msg_buffer).await.unwrap();
        let Ok(msg) = from_utf8(&msg_buffer) else {
            warn!("received invalid utf8 message from {from_addr}");
            continue;
        };

        info!("received message {msg} from {from_addr}");
    }
}

pub fn register(spawner: &Spawner, stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    spawner.spawn(handle(stack)).unwrap();
}
