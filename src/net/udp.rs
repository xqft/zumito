use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use log::{debug, warn};

const PORT: u16 = 8080;
const MSG_MAX_LEN: usize = 32;

pub static RX_MSG: Signal<CriticalSectionRawMutex, (usize, [u8; MSG_MAX_LEN])> = Signal::new();

#[embassy_executor::task]
async fn rx(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 1024];

    // empty tx buffers because this is rx only
    let mut udp_socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut [], &mut []);
    udp_socket.bind(PORT).unwrap();

    let mut msg_buffer = [0; MSG_MAX_LEN];
    loop {
        let Ok((msg_len, from_addr)) = udp_socket.recv_from(&mut msg_buffer).await else {
            warn!(
                "received message longer than msg buffer (which has length: {})",
                msg_buffer.len()
            );
            debug!("truncated message: {msg_buffer:?}");
            continue;
        };

        RX_MSG.signal((msg_len, msg_buffer));

        debug!(
            "received message {:?} from {from_addr}",
            &msg_buffer[..msg_len]
        );
    }
}

pub fn register(spawner: &Spawner, stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    spawner.spawn(rx(stack)).unwrap();
}
