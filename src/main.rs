use futures_util::future::join;
use jojo_server::initialize;
use log::*;
use std::net::Ipv4Addr;
use std::time::Duration;
use uuid::uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let (server_to_tauri_tx, mut server_to_tauri_rx) =
        tokio::sync::mpsc::channel::<jojo_common::room::RoomEvent>(32);

    let (tauri_to_client_tx, _) = tokio::sync::broadcast::channel(16);
    let tauri_to_client_tx_clone = tauri_to_client_tx.clone();

    let ip_local = Ipv4Addr::new(192, 168, 0, 163);
    let port = 3000;

    let task = initialize(ip_local, port, server_to_tauri_tx, tauri_to_client_tx);

    let server = tokio::spawn(task);

    let tauri_to_client_listener = tokio::spawn(async move {
        info!("[tauri_to_client_listener]: waiting 5s to send event");

        tokio::time::sleep(Duration::from_secs(50)).await;

        tauri_to_client_tx_clone
            .send(jojo_common::message::ServerMessage::RestartDevice(uuid!(
                "340917e8-87a9-455c-9645-d08eb99162f9"
            )))
            .unwrap();

        info!("[tauri_to_client_listener]: event send");
    });

    let server_to_tauri_listener = tokio::spawn(async move {
        info!("LISTENING");

        while let Some(event) = server_to_tauri_rx.recv().await {
            info!("EVENT EMITTED: {:?}", event)
        }
    });

    tauri_to_client_listener.await.unwrap();

    let (_, _) = join(server, server_to_tauri_listener).await;

    Ok(())
}
