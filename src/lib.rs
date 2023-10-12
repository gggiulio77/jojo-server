pub mod button;
pub mod db;
pub mod device;
pub mod discovery;
pub mod handler;
pub mod keyboard;
pub mod mouse;
pub mod room;

use anyhow::bail;
use handler::socket_handler;
use local_ip_address::local_ip;
use log::info;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

pub async fn initialize(sender: crossbeam_channel::Sender<room::RoomEvent>) -> anyhow::Result<()> {
    let bind_address: SocketAddr = match std::env::var("BROADCAST_BIND_ADDRESS") {
        Ok(value) => value.parse::<SocketAddr>()?,
        Err(_) => {
            bail!("[init]: BROADCAST_BIND_ADDRESS env not found")
        }
    };

    let my_local_ip = match local_ip()? {
        std::net::IpAddr::V4(ip) => ip,
        _ => {
            bail!("[init]: local IpV4 not found")
        }
    };

    let available_port = match get_available_port(my_local_ip) {
        Some(port) => {
            info!("[init]: port found {:?}", port);
            port
        }
        None => panic!("[init]: cannot find an available port on machine"),
    };

    info!(
        "[init]: server address: {:?}:{:?}",
        my_local_ip, available_port
    );

    discovery::init_broadcast(bind_address, available_port).await;

    let sender_filter = warp::any().map(move || sender.clone());

    // Simple in memory DB
    let devices = Arc::new(RwLock::new(db::DeviceMap::new()));

    let devices_filter = warp::any().map(move || devices.clone());

    let routes = warp::path("multiple")
        .and(warp::ws())
        .and(warp::path::param().map(|id: device::DeviceId| id))
        .and(devices_filter)
        .and(sender_filter)
        .map(|ws: warp::ws::Ws, user_id, devices, sender| {
            ws.on_upgrade(move |socket| async move {
                socket_handler(socket, user_id, devices, sender).await;

                info!("[ws]: closing thread");
            })
        });

    // TODO: replace hardcoded code with a dynamic one, find  way to know it so we can send it in the broadcast message
    warp::serve(routes).run((my_local_ip, available_port)).await;

    Ok(())
}

fn get_available_port(local_ip: Ipv4Addr) -> Option<u16> {
    (3000..9000).find(|port| port_is_available(local_ip, *port))
}

fn port_is_available(local_ip: Ipv4Addr, port: u16) -> bool {
    match TcpListener::bind((local_ip, port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
