use anyhow::bail;
use std::time::Duration;

use crate::{db, driver};
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use warp::ws::{Message, WebSocket};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Reads {
    mouse_read: Option<jojo_common::mouse::MouseRead>,
    button_reads: Option<Vec<jojo_common::button::ButtonRead>>,
}

impl Reads {
    pub fn new(
        mouse_read: Option<jojo_common::mouse::MouseRead>,
        button_reads: Option<Vec<jojo_common::button::ButtonRead>>,
    ) -> Self {
        Reads {
            mouse_read,
            button_reads,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum ClientMessage {
    Reads(Vec<Reads>),
    Device(jojo_common::device::Device),
}

pub async fn socket_handler(
    ws: WebSocket,
    device_id: jojo_common::device::DeviceId,
    devices: db::Devices,
    sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
) {
    let (mut tx, rx) = ws.split();
    // let (timeout_tx, timeout_rx) = crossbeam_channel::unbounded::<Instant>();
    // Timeout channel
    let (timeout_tx, mut timeout_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Exit socket channel
    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let exit_tx_2 = exit_tx.clone();

    // Msg sender channel
    let (sender_tx, mut sender_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let mouse_driver = driver::mouse::MouseDriver::default();
    let button_driver = driver::button::ButtonDriver::default();

    let ping_sender = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(700));
        loop {
            interval.tick().await;
            // info!("[ws]: sending ping");
            sender_tx.send(Message::ping("")).unwrap();
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
        exit_tx.send(()).unwrap();
    });

    let msg_sender = tokio::spawn(async move {
        while let Some(msg) = sender_rx.recv().await {
            match tx.send(msg).await {
                Ok(_) => {}
                Err(err) => {
                    error!("[ws]: cannot send msg, err: {}", err);
                    break;
                }
            };
        }
    });

    let devices_cloned = devices.clone();
    let sender_cloned = sender.clone();

    // TODO: find a way to propagate errors
    let read_socket = tokio::spawn(async move {
        ws_message_handler(
            devices_cloned,
            rx,
            timeout_tx,
            exit_tx_2,
            mouse_driver,
            button_driver,
            sender_cloned,
        )
        .await
    });

    exit_rx.recv().await.unwrap();

    info!("[ws]: closing thread");

    read_socket.abort();
    ping_sender.abort();
    timeout_task.abort();
    msg_sender.abort();

    devices.write().await.remove(&device_id, sender);
}

async fn ws_message_handler(
    devices: db::Devices,
    mut rx: SplitStream<WebSocket>,
    timeout_tx: UnboundedSender<()>,
    exit_tx_2: UnboundedSender<()>,
    mut mouse_driver: driver::mouse::MouseDriver,
    mut button_driver: driver::button::ButtonDriver,
    sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
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
            timeout_tx.send(()).unwrap();
            continue;
        }

        if msg.is_close() {
            info!("[ws]: close message received");
            exit_tx_2.send(()).unwrap();
            break;
        }

        if msg.is_binary() {
            match bincode::deserialize::<ClientMessage>(msg.as_bytes()) {
                Ok(client_message) => {
                    client_message_handler(
                        client_message,
                        &mut mouse_driver,
                        &mut button_driver,
                        devices.clone(),
                        sender.clone(),
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
                        &mut mouse_driver,
                        &mut button_driver,
                        devices.clone(),
                        sender.clone(),
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

async fn client_message_handler(
    client_message: ClientMessage,
    mouse_driver: &mut driver::mouse::MouseDriver,
    button_driver: &mut driver::button::ButtonDriver,
    devices: db::Devices,
    sender: crossbeam_channel::Sender<jojo_common::room::RoomEvent>,
) {
    match client_message {
        ClientMessage::Reads(reads) => {
            // info!("[ws]: evaluating reads");
            for read in &reads {
                if let Some(mouse_read) = read.mouse_read {
                    // info!("[ws]: mouse read");
                    let (x_read, y_read) = (mouse_read.x_read(), mouse_read.y_read());
                    mouse_driver.mouse_move_relative(x_read, y_read);
                }
                if let Some(button_reads) = &read.button_reads {
                    for button_read in button_reads {
                        match button_read.kind() {
                            jojo_common::button::Button::MouseButton(mouse_button) => {
                                button_driver.button_to_state(mouse_button);
                            }
                            _ => todo!(),
                        }
                    }
                }
            }
        }
        ClientMessage::Device(device) => {
            // info!("[ws]: saving device {}", device.id());

            devices
                .write()
                .await
                .insert(device.id(), device, sender.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::uuid;

    #[test]
    fn test_serialize_message() {
        let device_msg = r#"{"Device": {"id": "340917e8-87a9-455c-9645-d08eb99162f9","name": "tu_vieja","mouse_config": null,"buttons": []}}"#;
        let reads_msg =
            r#"{"Reads": [{"mouse_read": {"x_read": 100, "y_read": 100}, "button_reads": null}]}"#;
        let id = uuid!("340917e8-87a9-455c-9645-d08eb99162f9");

        let device_result: ClientMessage = serde_json::from_str(device_msg).unwrap();
        let reads_result: ClientMessage = serde_json::from_str(reads_msg).unwrap();

        assert_eq!(
            device_result,
            ClientMessage::Device(jojo_common::device::Device::new(
                id,
                format!("tu_vieja"),
                None,
                Vec::new()
            ))
        );

        assert_eq!(
            reads_result,
            ClientMessage::Reads(vec![Reads::new(
                Some(jojo_common::mouse::MouseRead::new(100, 100)),
                None
            )])
        );
    }
}
