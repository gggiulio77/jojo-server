use crate::mouse::{Mouse, MouseRead};
use futures_util::StreamExt;
use log::*;
use warp::ws::WebSocket;

pub async fn single(ws: WebSocket, mut mouse: Mouse) {
    let (mut _tx, mut rx) = ws.split();

    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error: {}", e);
                break;
            }
        };

        if msg.is_close() {
            info!("closing socket");
            break;
        }

        if msg.is_binary() {
            match bincode::deserialize::<MouseRead>(msg.as_bytes()) {
                Ok(MouseRead { x_read, y_read }) => {
                    mouse.mouse_move_relative(
                        x_read * mouse.x_sen as i32,
                        y_read * mouse.y_sen as i32,
                    );
                }

                Err(err) => {
                    error!("websocket error: {}", err);
                    break;
                }
            }
        }

        if msg.is_text() {
            match serde_json::from_str::<MouseRead>(msg.to_str().unwrap()) {
                Ok(MouseRead { x_read, y_read }) => {
                    mouse.mouse_move_relative(
                        x_read * mouse.x_sen as i32,
                        y_read * mouse.y_sen as i32,
                    );
                }

                Err(err) => {
                    error!("websocket error: {}", err);
                    break;
                }
            }
        }
    }
}

pub async fn multiple(ws: WebSocket, mut mouse: Mouse) {
    let (mut _tx, mut rx) = ws.split();

    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error: {}", e);
                break;
            }
        };

        if msg.is_close() {
            info!("closing socket");
            break;
        }

        if msg.is_binary() {
            match bincode::deserialize::<Vec<MouseRead>>(msg.as_bytes()) {
                Ok(reads) => {
                    for MouseRead { x_read, y_read } in &reads {
                        mouse.mouse_move_relative(
                            x_read * mouse.x_sen as i32,
                            y_read * mouse.y_sen as i32,
                        );
                    }
                }

                Err(err) => {
                    error!("websocket error: {}", err);
                    break;
                }
            }
        }

        if msg.is_text() {
            match serde_json::from_str::<Vec<MouseRead>>(msg.to_str().unwrap()) {
                Ok(reads) => {
                    for MouseRead { x_read, y_read } in &reads {
                        mouse.mouse_move_relative(
                            x_read * mouse.x_sen as i32,
                            y_read * mouse.y_sen as i32,
                        );
                    }
                }

                Err(err) => {
                    error!("websocket error: {}", err);
                    break;
                }
            }
        }
    }
}
