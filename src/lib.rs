pub mod db;
pub mod handler;

use handler::socket_handler;
use log::info;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

pub async fn initialize(
    ip_address: Ipv4Addr,
    port: u16,
    sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
) -> anyhow::Result<()> {
    info!("[init]: server address: {:?}:{:?}", ip_address, port);

    let sender_filter = warp::any().map(move || sender.clone());

    // Simple in memory DB
    let devices = Arc::new(RwLock::new(db::DeviceMap::new()));

    let devices_filter = warp::any().map(move || devices.clone());

    let routes = warp::path("multiple")
        .and(warp::ws())
        .and(warp::path::param().map(|id: jojo_common::device::DeviceId| id))
        .and(devices_filter)
        .and(sender_filter)
        .map(|ws: warp::ws::Ws, device_id, devices, sender| {
            // TODO: receive an Receiver<Something> to listen to events on the tauri side of things
            // TODO: implement a task to listen to tauri events for our device_id, send it to the device via websockets
            ws.on_upgrade(move |socket| socket_handler(socket, device_id, devices, sender))
        });

    warp::serve(routes).run((ip_address, port)).await;

    Ok(())
}
