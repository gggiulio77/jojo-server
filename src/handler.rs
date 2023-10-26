use anyhow::bail;
use std::time::Duration;

use crate::db;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use jojo_common::button::ButtonRead;
use jojo_common::device::DeviceId;
use jojo_common::driver::button::ButtonDriver;
use jojo_common::driver::mouse::MouseDriver;
use jojo_common::keyboard::KeyboardButton;
use jojo_common::message::{ClientMessage, Reads};
use jojo_common::room::RoomEvent;
use log::*;
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;
use warp::ws::{Message, WebSocket};

pub async fn socket_handler(
    ws: WebSocket,
    device_id: DeviceId,
    devices: db::Devices,
    tauri_sender_tx: crossbeam_channel::Sender<RoomEvent>,
) {
    let (mut tx, rx) = ws.split();

    // Timeout channel
    let (timeout_tx, mut timeout_rx) = tokio::sync::mpsc::channel::<()>(32);

    // Exit socket channel
    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::channel::<()>(32);
    let exit_tx_2 = exit_tx.clone();

    // Ws msg sender channel
    let (ws_sender_tx, mut ws_sender_rx) = tokio::sync::mpsc::channel::<Message>(32);

    // button::Reads channel
    let (read_sender_tx, mut read_sender_rx) = tokio::sync::mpsc::channel::<Vec<Reads>>(32);

    // This task is necessary because enigo was blocking all tokio tasks
    let read_handler = tokio::spawn(async move {
        while let Some(reads) = read_sender_rx.recv().await {
            tokio::task::block_in_place(move || {
                let duration = Instant::now();
                info!("[read_handler]: Handling msg");
                read_handler(reads);
                info!("[read_handler]: Handed msg, {:?}", duration.elapsed());
            });
        }
    });

    let ping_sender = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(700));
        loop {
            interval.tick().await;
            ws_sender_tx.send(Message::ping("")).await.unwrap();
        }
    });

    // TODO: rewrite this timeout_task, it make me sick
    let timeout_task = tokio::spawn(async move {
        loop {
            if let Err(_) = tokio::time::timeout(Duration::from_secs(1), async {
                if let None = timeout_rx.recv().await {
                    return;
                }
            })
            .await
            {
                break;
            }
        }
        info!("[ws]: closing connection due to 3s timeout");
        exit_tx.send(()).await.unwrap();
    });

    let msg_sender = tokio::spawn(async move {
        while let Some(msg) = ws_sender_rx.recv().await {
            match tx.send(msg).await {
                Ok(_) => {}
                Err(err) => {
                    error!("[ws]: cannot send msg, err: {}", err);
                    break;
                }
            };
        }
    });

    let devices_clone = devices.clone();
    let tauri_sender_tx_clone = tauri_sender_tx.clone();

    // TODO: find a way to propagate errors
    let read_socket = tokio::spawn(async move {
        ws_message_handler(
            rx,
            timeout_tx,
            exit_tx_2,
            devices_clone,
            read_sender_tx,
            tauri_sender_tx_clone,
        )
        .await
    });

    exit_rx.recv().await.unwrap();

    info!("[ws]: closing thread");

    read_socket.abort();
    ping_sender.abort();
    timeout_task.abort();
    msg_sender.abort();
    read_handler.abort();

    devices.write().await.remove(&device_id, tauri_sender_tx);
}

async fn ws_message_handler(
    mut rx: SplitStream<WebSocket>,
    timeout_tx: Sender<()>,
    exit_tx_2: Sender<()>,
    devices: db::Devices,
    read_sender_tx: Sender<Vec<Reads>>,
    tauri_sender_tx: crossbeam_channel::Sender<RoomEvent>,
) -> Result<(), anyhow::Error> {
    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(err) => {
                error!("[ws]: message error: {}", err);
                bail!("[ws]: message error: {}", err);
            }
        };

        if msg.is_pong() {
            // info!("[ws]: pong received from");
            timeout_tx.send(()).await.unwrap();
            continue;
        }

        if msg.is_close() {
            info!("[ws]: close message received");
            exit_tx_2.send(()).await.unwrap();
            break;
        }

        if msg.is_binary() {
            match bincode::deserialize::<ClientMessage>(msg.as_bytes()) {
                Ok(client_message) => {
                    client_message_handler(
                        client_message,
                        &devices,
                        &read_sender_tx,
                        &tauri_sender_tx,
                    )
                    .await
                }
                Err(err) => {
                    error!("[ws]: binary error: {}", err);
                    bail!("[ws]: binary error: {}", err);
                }
            }
        }

        if msg.is_text() {
            match serde_json::from_str::<ClientMessage>(msg.to_str().unwrap()) {
                Ok(client_message) => {
                    client_message_handler(
                        client_message,
                        &devices,
                        &read_sender_tx,
                        &tauri_sender_tx,
                    )
                    .await
                }
                Err(err) => {
                    error!("[ws]: text error: {}", err);
                    bail!("[ws]: text error: {}", err);
                }
            }
        }
    }
    Ok(())
}

fn read_handler(reads: Vec<Reads>) {
    // TODO: think about re using drivers instances, instead of creating with each message
    let mut mouse_driver = MouseDriver::default();
    let mut button_driver = ButtonDriver::default();
    for read in &reads {
        if let Some(mouse_read) = read.mouse_read() {
            // info!("[ws]: mouse read");
            let (x_read, y_read) = (mouse_read.x_read(), mouse_read.y_read());
            mouse_driver.mouse_move_relative(x_read, y_read);
        }
        if let Some(button_reads) = read.button_reads() {
            for button_read in button_reads {
                match button_read {
                    ButtonRead::MouseButton(mouse_button) => {
                        button_driver.mouse_button_to_state(mouse_button);
                    }
                    ButtonRead::KeyboardButton(keyboard_button) => match keyboard_button {
                        KeyboardButton::Sequence(sequence) => button_driver.key_sequence(sequence),
                        KeyboardButton::SequenceDsl(sequence) => {
                            button_driver.key_sequence_dsl(sequence)
                        }
                        KeyboardButton::Key(key) => button_driver.key_click(key.to_owned()),
                    },
                    _ => todo!(),
                }
            }
        }
    }
}
async fn client_message_handler(
    client_message: ClientMessage,
    devices: &db::Devices,
    read_sender_tx: &Sender<Vec<Reads>>,
    sender: &crossbeam_channel::Sender<RoomEvent>,
) {
    match client_message {
        ClientMessage::Reads(reads) => read_sender_tx.send(reads).await.unwrap(),
        ClientMessage::Device(device) => {
            // info!("[ws]: saving device {}", device.id());
            devices
                .write()
                .await
                .insert(device.id(), device, sender.clone());
        }
    }
}
