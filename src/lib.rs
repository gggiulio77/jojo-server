pub mod db;
pub mod handler;

use crate::db::Devices;
use axum::extract::{Path, State};
use axum::{extract::ws::WebSocketUpgrade, routing::get, Router};
use jojo_common::device::DeviceId;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    devices: Devices,
    sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
}

pub async fn initialize(
    ip_address: Ipv4Addr,
    port: u16,
    sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
) {
    let devices = Arc::new(RwLock::new(db::DeviceMap::new()));
    let shared_state = AppState { devices, sender };

    let app =
        Router::new()
            .route(
                "/ws/:id",
                get(
                    |Path(id): Path<DeviceId>,
                     ws: WebSocketUpgrade,
                     State(state): State<AppState>| async move {
                        ws.on_upgrade(move |socket| {
                            handler::socket_handler(
                                socket,
                                id,
                                state.devices.clone(),
                                state.sender.clone(),
                            )
                        })
                    },
                ),
            )
            .with_state(shared_state);

    axum::Server::bind(&(ip_address, port).into())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
