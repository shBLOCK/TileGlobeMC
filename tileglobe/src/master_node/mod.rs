use core::str::from_utf8;
use defmt_or_log::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embedded_io_async::Write;

const MAX_CLIENTS: usize = 3;
const RX_BUTTER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;

#[embassy_executor::task(pool_size = MAX_CLIENTS)]
async fn client_task(net_stack: embassy_net::Stack<'static>) {
    let mut rx_buffer = [0; RX_BUTTER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut buffer = [0; 4096];
    loop {
        let mut socket = TcpSocket::new(net_stack, &mut rx_buffer, &mut tx_buffer);
        if let Err(err) = socket.accept(25565).await {
            warn!("TCP accept failed: {:?}", err);
            continue;
        }
        let ip = socket.remote_endpoint().unwrap();
        info!("Connected to {:?}", ip);

        loop {
            let n = match socket.read(&mut buffer).await {
                Ok(0) => {
                    warn!("Read EOF");
                    break;
                }
                Ok(n) => n,
                Err(err) => {
                    warn!("Read failed: {:?}", err);
                    break;
                }
            };

            info!("Read {} bytes", n);
            info!("Data: {}", from_utf8(&buffer[..n]).unwrap());

            if let Err(err) = socket.write_all(&buffer[..n]).await {
                warn!("Write failed: {:?}", err);
                break;
            }
        }

        info!("Disconnecting from {:?}", ip);
        socket.abort();
        _ = socket.flush().await;
    }
}

pub async fn master_node_start(spawner: Spawner, net_stack: embassy_net::Stack<'static>) {
    for _ in 0..MAX_CLIENTS {
        spawner.spawn(client_task(net_stack)).unwrap();
    }
}