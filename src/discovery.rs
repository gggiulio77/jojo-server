use log::*;
use tokio::net::{ToSocketAddrs, UdpSocket};

pub async fn init_broadcast<A: ToSocketAddrs>(bind_address: A, server_port: u16) {
    let socket = UdpSocket::bind(bind_address).await.unwrap();
    socket.set_broadcast(true).unwrap();

    info!("listening UDP on {:?}", socket.local_addr().unwrap());

    tokio::spawn(async move {
        let mut buffer = [0; 512];
        loop {
            // TODO: think a way to make this more secure, maybe encrypt payload with a date or something and encrypt/decrypt in both ends
            let (len, addr) = socket.recv_from(&mut buffer).await.unwrap();
            info!("{:?} bytes received from {:?}", len, addr);

            let len = socket
                .send_to(&server_port.to_string().as_bytes(), addr)
                .await
                .unwrap();
            info!("{:?} bytes sent", len);
        }
    });
}

// TODO: implement the multicast version
