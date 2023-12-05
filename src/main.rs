use futures_util::future::join;
use jojo_common::button::ButtonAction;
use jojo_common::command::CustomCommand;
use jojo_common::keyboard::{Key, KeyboardButton};
use jojo_server::initialize;
use log::*;
use std::collections::HashMap;
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

    // let tauri_to_client_listener = tokio::spawn(async move {
    //     info!("[tauri_to_client_listener]: waiting 5s to send event");

    //     tokio::time::sleep(Duration::from_secs(10)).await;

    //     let button_actions: Vec<jojo_common::button::ButtonAction> = vec![
    //         ButtonAction::CustomButton(CustomCommand::Binary(
    //             "C:\\Users\\gggiu\\AppData\\Roaming\\Spotify\\Spotify.exe".to_string(),
    //         )),
    //         ButtonAction::KeyboardButton(KeyboardButton::Key(Key::Space)),
    //     ];

    //     // tauri_to_client_tx_clone
    //     //     .send(jojo_common::message::ServerMessage::RestartDevice(uuid!(
    //     //         "58c79037-d101-476d-bcbe-1503e9011261"
    //     //     )))
    //     //     .unwrap();

    //     let message = jojo_common::message::ServerMessage::UpdateDevice(
    //         uuid!("58c79037-d101-476d-bcbe-1503e9011261"),
    //         HashMap::from([(
    //             uuid!("0ce7ecdb-4dcc-46f5-804c-65a39d2277a0"),
    //             button_actions,
    //         )]),
    //     );

    //     tauri_to_client_tx_clone.send(message).unwrap();

    //     info!("[tauri_to_client_listener]: event send");
    // });

    let server_to_tauri_listener = tokio::spawn(async move {
        info!("LISTENING");

        while let Some(event) = server_to_tauri_rx.recv().await {
            info!("EVENT EMITTED: {:?}", event)
        }
    });

    // tauri_to_client_listener.await.unwrap();

    let (_, _) = join(server, server_to_tauri_listener).await;

    Ok(())
}
