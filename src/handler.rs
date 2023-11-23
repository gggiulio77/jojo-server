use axum::extract::ws::{Message, WebSocket};
use std::time::Duration;

use crate::db;
use futures_util::future::join_all;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use jojo_common::button::ButtonAction;
use jojo_common::device::DeviceId;
use jojo_common::driver::button::ButtonDriver;
use jojo_common::driver::mouse::MouseDriver;
use jojo_common::keyboard::KeyboardButton;
use jojo_common::message::{ClientMessage, Reads};
use jojo_common::room::RoomEvent;
use log::*;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

const TIMEOUT_MILLIS: u64 = 10_000;
const PING_MILLIS: u64 = 5_000;

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
    let exit_tx_3 = exit_tx.clone();

    // Ws msg sender channel
    let (ws_sender_tx, mut ws_sender_rx) = tokio::sync::mpsc::channel::<Message>(32);

    // button::Reads channel
    let (read_sender_tx, mut read_sender_rx) = tokio::sync::mpsc::channel::<Vec<Reads>>(32);

    // This task is necessary because enigo was blocking all tokio tasks
    let read_handler = tokio::spawn(async move {
        while let Some(reads) = read_sender_rx.recv().await {
            tokio::spawn(async move {
                // let duration = Instant::now();
                // info!("[read_handler]: Handling msg");
                read_handler(reads).await;
                // info!("[read_handler]: Handed msg, {:?}", duration.elapsed());
            });
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

    let msg_sender = tokio::spawn(async move {
        while let Some(msg) = ws_sender_rx.recv().await {
            match tx.send(msg).await {
                Ok(_) => {}
                Err(err) => {
                    error!("[ws]: cannot send msg, err: {}", err);
                    exit_tx_3
                        .send(())
                        .await
                        .unwrap_or_else(|_| info!("[timeout_task]: exit_tx send error"));
                    break;
                }
            };
        }
    });

    // TODO: rewrite this timeout_task, it make me sick
    let timeout_task = tokio::spawn(async move {
        loop {
            if let Err(_) = tokio::time::timeout(Duration::from_millis(TIMEOUT_MILLIS), async {
                if let Some(_) = timeout_rx.recv().await {
                    return;
                }
            })
            .await
            {
                break;
            }
        }
        info!("[ws]: closing connection due to {TIMEOUT_MILLIS}ms timeout");
        exit_tx
            .send(())
            .await
            .unwrap_or_else(|_| info!("[timeout_task]: exit_tx send error"));
    });

    let ping_sender = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(PING_MILLIS));
        loop {
            interval.tick().await;
            ws_sender_tx
                .send(Message::Ping(vec![]))
                .await
                .unwrap_or_else(|_| info!("[ping_sender]: ws_sender_tx send error"));
        }
    });

    exit_rx
        .recv()
        .await
        .unwrap_or_else(|| info!("[timeout_task]: recv send error"));

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
                // TODO: review what to do in this case, maybe we can close the socket
                error!("[ws]: message error: {}", err);
                Message::Pong(vec![])
                // bail!("[ws]: message error: {}", err);
            }
        };

        match msg {
            Message::Pong(_) => {
                // info!("[ws]: pong received from");
                timeout_tx
                    .send(())
                    .await
                    .unwrap_or_else(|_| info!("[msg.is_pong()]: timeout_tx send error"));
            }
            Message::Ping(_) => {
                info!("[ws]: ping received from");
            }
            Message::Close(_) => {
                info!("[ws]: close message received");
                exit_tx_2
                    .send(())
                    .await
                    .unwrap_or_else(|_| info!("[msg.is_close()]: exit_tx_2 send error"));
                break;
            }
            Message::Text(message) => {
                match serde_json::from_str::<ClientMessage>(&message) {
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
                        // TODO: this error exist when the payload is bad, for now we are ignoring it
                        error!("[ws]: deserialize text: {}", err);
                        // bail!("[ws]: text error: {}", err);
                    }
                }
            }
            Message::Binary(message) => {
                match bincode::deserialize::<ClientMessage>(&message) {
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
                        // TODO: this error exist when the payload is bad, for now we are ignoring it
                        error!("[ws]: deserialize binary: {}", err);
                        // bail!("[ws]: binary error: {}", err);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn read_handler(reads: Vec<Reads>) {
    let mut tasks: Vec<JoinHandle<()>> = vec![];

    for read in reads {
        // TODO: think about re using drivers instances, instead of creating with each message
        let (mouse_read, button_actions) = (
            read.mouse_read().to_owned(),
            read.button_actions().to_owned(),
        );

        if let Some(mouse_read) = mouse_read {
            tasks.push(tokio::task::spawn_blocking(move || {
                let mut mouse_driver = MouseDriver::default();
                let (x_read, y_read) = (mouse_read.x_read(), mouse_read.y_read());

                let mut x_total = x_read.abs();
                let mut y_total = y_read.abs();

                (0..x_total.max(y_total)).for_each(|_| {
                    match (x_total, y_total) {
                        (0, _) => {
                            mouse_driver.mouse_move_relative(0, 1 * y_read.signum());
                            y_total -= 1;
                        }
                        (_, 0) => {
                            mouse_driver.mouse_move_relative(1 * x_read.signum(), 0);
                            x_total -= 1;
                        }
                        (_, _) => {
                            mouse_driver
                                .mouse_move_relative(1 * x_read.signum(), 1 * y_read.signum());
                            y_total -= 1;
                            x_total -= 1;
                        }
                    }
                    spin_sleep::sleep(Duration::from_micros(2000));
                });
            }));
        };

        if let Some(button_actions) = button_actions {
            tasks.push(tokio::task::spawn_blocking(move || {
                let mut button_driver = ButtonDriver::default();

                for button_action in button_actions {
                    info!("[read_handler]: {:?}", button_action);
                    match button_action {
                        ButtonAction::MouseButton(mouse_button, state) => {
                            button_driver
                                .mouse_button_to_state(mouse_button.to_owned(), state.to_owned());
                        }
                        ButtonAction::KeyboardButton(keyboard_button) => match keyboard_button {
                            KeyboardButton::Sequence(sequence) => {
                                button_driver.key_sequence(&sequence)
                            }
                            KeyboardButton::SequenceDsl(sequence) => {
                                button_driver.key_sequence_dsl(&sequence)
                            }
                            KeyboardButton::Key(key) => button_driver.key_click(key.to_owned()),
                        },
                        _ => todo!(),
                    }
                }
            }));
        }
    }

    join_all(tasks).await;
}
async fn client_message_handler(
    client_message: ClientMessage,
    devices: &db::Devices,
    read_sender_tx: &Sender<Vec<Reads>>,
    sender: &crossbeam_channel::Sender<RoomEvent>,
) {
    match client_message {
        ClientMessage::Reads(reads) => read_sender_tx
            .send(reads)
            .await
            .unwrap_or_else(|_| info!("[client_message_handler]: read_sender_tx send error")),
        ClientMessage::Device(device) => {
            // info!("[ws]: saving device {}", device.id());
            devices
                .write()
                .await
                .insert(device.id(), device, sender.clone());
        }
    }
}
