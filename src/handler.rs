use axum::extract::ws::{Message, WebSocket};

use jojo_common::driver::gamepad::GamePadAdapter;
use jojo_common::driver::gamepad::GamepadDriver;
use jojo_common::driver::keyboard::KeyboardDriver;
use jojo_common::gamepad::AxisRead;
use jojo_common::gamepad::HatRead;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::Duration;

use crate::db;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use jojo_common::button::ButtonAction;
use jojo_common::command::{CommandDriver, CustomCommand};
use jojo_common::device::DeviceId;
use jojo_common::driver::mouse::MouseDriver;
use jojo_common::keyboard::KeyboardButton;
use jojo_common::message::{ClientMessage, ServerMessage};
use jojo_common::room::RoomEvent;
use log::*;
const TIMEOUT_MILLIS: u64 = 10_000;
const PING_MILLIS: u64 = 5_000;

lazy_static! {
    // TODO: think about replace this with an Arc and passing drivers as a tuple down the functions. Or with OnceCell
    // TODO: review a bug involving USB and vjoy, for some reason while connecting and disconnecting the esp the server loose ownership of the vjoy device and cannot upload anymore
    static ref MOUSE_DRIVER_STACK: Mutex<MouseDriver> = Mutex::new(MouseDriver::default());
    static ref KEYBOARD_DRIVER_STACK: Mutex<KeyboardDriver> = Mutex::new(KeyboardDriver::default());
    static ref GAMEPAD_DRIVER_STACK: Mutex<GamepadDriver> = Mutex::new(GamepadDriver::default());
}

pub async fn socket_handler(
    ws: WebSocket,
    device_id: DeviceId,
    devices: db::Devices,
    server_to_tauri_tx: tokio::sync::mpsc::Sender<RoomEvent>,
    mut tauri_to_client_rx: tokio::sync::broadcast::Receiver<ServerMessage>,
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
    let tauri_ws_sender_tx = ws_sender_tx.clone();

    let devices_clone = devices.clone();
    let tauri_sender_tx_clone = server_to_tauri_tx.clone();

    let read_tauri = tokio::spawn(async move {
        while let Ok(msg) = tauri_to_client_rx.recv().await {
            // TODO: need refactor, doing the same thing in all cases
            match msg {
                ServerMessage::UpdateDevice(msg_device_id, button_actions)
                    if msg_device_id == device_id =>
                {
                    let message =
                        bincode::serialize(&ServerMessage::UpdateDevice(device_id, button_actions))
                            .expect("[read_tauri]: cannot serialize");

                    tauri_ws_sender_tx
                        .send(Message::Binary(message))
                        .await
                        .expect("[read_tauri]: cannot send message");
                }
                ServerMessage::RestartDevice(msg_device_id) if msg_device_id == device_id => {
                    let message = bincode::serialize(&ServerMessage::RestartDevice(device_id))
                        .expect("[read_tauri]: cannot serialize");

                    tauri_ws_sender_tx
                        .send(Message::Binary(message))
                        .await
                        .expect("[read_tauri]: cannot send message");
                }
                ServerMessage::ClearCredentials(msg_device_id) if msg_device_id == device_id => {
                    let message = bincode::serialize(&ServerMessage::ClearCredentials(device_id))
                        .expect("[read_tauri]: cannot serialize");

                    tauri_ws_sender_tx
                        .send(Message::Binary(message))
                        .await
                        .expect("[read_tauri]: cannot send message");
                }
                _ => info!("[read_tauri]: msg not for us"),
            }
        }
    });

    // TODO: find a way to propagate errors
    let read_socket = tokio::spawn(async move {
        ws_message_handler(
            rx,
            timeout_tx,
            exit_tx_2,
            devices_clone,
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
            if tokio::time::timeout(Duration::from_millis(TIMEOUT_MILLIS), async {
                if timeout_rx.recv().await.is_some() {}
            })
            .await
            .is_err()
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
        let mut interval = tokio::time::interval(Duration::from_millis(PING_MILLIS));
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

    read_tauri.abort();
    read_socket.abort();
    ping_sender.abort();
    timeout_task.abort();
    msg_sender.abort();

    devices
        .write()
        .await
        .remove(&device_id, server_to_tauri_tx)
        .await;
}

async fn ws_message_handler(
    mut rx: SplitStream<WebSocket>,
    timeout_tx: tokio::sync::mpsc::Sender<()>,
    exit_tx_2: tokio::sync::mpsc::Sender<()>,
    devices: db::Devices,
    tauri_sender_tx: tokio::sync::mpsc::Sender<RoomEvent>,
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
                        client_message_handler(client_message, &devices, &tauri_sender_tx).await
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
                        client_message_handler(client_message, &devices, &tauri_sender_tx).await
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

async fn client_message_handler(
    client_message: ClientMessage,
    devices: &db::Devices,
    sender: &tokio::sync::mpsc::Sender<RoomEvent>,
) {
    // TODO: use references for drivers, drivers are mutable, so we need a lock or channels to handle multi tasks
    // TODO: Device is an async task, but the rest of the types are sync threads, find a way to re write this

    match client_message {
        ClientMessage::MouseRead(mouse_read) => {
            tokio::task::spawn_blocking(move || {
                let (x_read, y_read) = (mouse_read.x_read(), mouse_read.y_read());

                let mut x_total = x_read.abs();
                let mut y_total = y_read.abs();
                let max = x_total.max(y_total);

                // Calculate delay in ms, we are sending mouse_read in 150ms intervals
                let wait: u64 = 150 / max as u64;

                let mut mouse_driver = MOUSE_DRIVER_STACK.lock().unwrap();
                (0..max).for_each(|_| {
                    match (x_total, y_total) {
                        (0, _) => {
                            mouse_driver.mouse_move_relative(0, y_read.signum());
                            y_total -= 1;
                        }
                        (_, 0) => {
                            mouse_driver.mouse_move_relative(x_read.signum(), 0);
                            x_total -= 1;
                        }
                        (_, _) => {
                            mouse_driver.mouse_move_relative(x_read.signum(), y_read.signum());
                            y_total -= 1;
                            x_total -= 1;
                        }
                    }
                    spin_sleep::sleep(Duration::from_millis(wait));
                });
            })
            .await
            .expect("[mouse_read]: fail case");
        }
        ClientMessage::ButtonActions(button_actions) => {
            tokio::task::spawn_blocking(move || {
                for button_action in button_actions {
                    info!("[client_message_handler]: {:?}", button_action);
                    match button_action {
                        ButtonAction::MouseButton(mouse_button, state) => {
                            MOUSE_DRIVER_STACK
                                .lock()
                                .unwrap()
                                .mouse_button_to_state(mouse_button.to_owned(), state.to_owned());
                        }
                        ButtonAction::KeyboardButton(keyboard_button) => match keyboard_button {
                            KeyboardButton::Sequence(sequence) => KEYBOARD_DRIVER_STACK
                                .lock()
                                .unwrap()
                                .key_sequence(&sequence),
                            KeyboardButton::SequenceDsl(sequence) => KEYBOARD_DRIVER_STACK
                                .lock()
                                .unwrap()
                                .key_sequence_parse(&sequence),
                            KeyboardButton::Key(key) => KEYBOARD_DRIVER_STACK
                                .lock()
                                .unwrap()
                                .key_click(key.to_owned()),
                        },
                        ButtonAction::GamepadButton(gamepad_button, state) => GAMEPAD_DRIVER_STACK
                            .lock()
                            .unwrap()
                            .gamepad_button_to_state(gamepad_button, state),
                        ButtonAction::CustomButton(command) => match command {
                            // TODO: run_binary does his job, but if we want to concatenate a Keyboard command with this
                            // a "focus" issue appears. Windows focus a program when is started from the cmd. But the timing if the program already exist and
                            // the program never opened is different, so if we need to send a Key to this program we need to find a way to ensure that the focus
                            // is en that program
                            CustomCommand::Binary(path) => {
                                CommandDriver::run_binary(path)
                                    .expect("[command_driver]: failed to run binary");
                                std::thread::sleep(Duration::from_millis(1500));
                            }
                        },
                    }
                }
            })
            .await
            .expect("[button_actions]: fail case");
        }
        ClientMessage::AxisRead(axis_read) => {
            tokio::task::spawn_blocking(move || {
                info!("[client_message_handler]: {:?}", axis_read);

                let AxisRead(axis, value) = axis_read;

                GAMEPAD_DRIVER_STACK
                    .lock()
                    .unwrap()
                    .set_axis(axis, value)
                    .unwrap();
            })
            .await
            .expect("[axis_read]: fail case");
        }
        ClientMessage::HatRead(hat_read) => {
            tokio::task::spawn_blocking(move || {
                info!("[client_message_handler]: {:?}", hat_read);

                let HatRead(hat, value) = hat_read;

                GAMEPAD_DRIVER_STACK
                    .lock()
                    .unwrap()
                    .set_hat(hat, value)
                    .unwrap();
            })
            .await
            .expect("[hat_read]: fail case");
        }
        ClientMessage::Device(device) => {
            // info!("[ws]: saving device {}", device.id());
            devices
                .write()
                .await
                .insert(device.id(), device, sender.clone())
                .await;
        }
    }
}
