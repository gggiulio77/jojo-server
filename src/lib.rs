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
    server_tauri_tx: tokio::sync::mpsc::Sender<jojo_common::room::RoomEvent>,
    tauri_client_tx: tokio::sync::broadcast::Sender<jojo_common::message::ServerMessage>,
}

pub async fn initialize(
    ip_address: Ipv4Addr,
    port: u16,
    server_tauri_tx: tokio::sync::mpsc::Sender<jojo_common::room::RoomEvent>,
    tauri_client_tx: tokio::sync::broadcast::Sender<jojo_common::message::ServerMessage>,
) {
    let devices = Arc::new(RwLock::new(db::DeviceMap::new()));
    let shared_state = AppState {
        devices,
        server_tauri_tx,
        tauri_client_tx,
    };

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
                                state.server_tauri_tx.clone(),
                                state.tauri_client_tx.subscribe(),
                            )
                        })
                    },
                ),
            )
            .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind((ip_address, port))
        .await
        .unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
