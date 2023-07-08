use crate::mouse::{Mouse, MouseRead};
use futures_util::StreamExt;
use log::*;
use warp::ws::WebSocket;

pub async fn multiple(ws: WebSocket, mut mouse: Mouse) {
    let (mut _tx, mut rx) = ws.split();

    let (x_sen, y_sen) = mouse.sensibility();

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
                    for read in &reads {
                        let (x_read, y_read, click_read) = read.reads();

                        mouse.mouse_move_relative(x_read * x_sen as i32, y_read * y_sen as i32);
                        if click_read {
                            mouse.mouse_move_up(enigo::MouseButton::Left);
                        } else {
                            mouse.mouse_move_down(enigo::MouseButton::Left);
                        }
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
                    for read in &reads {
                        let (x_read, y_read, click_read) = read.reads();

                        mouse.mouse_move_relative(x_read * x_sen as i32, y_read * y_sen as i32);
                        if click_read {
                            mouse.mouse_move_up(enigo::MouseButton::Left);
                        } else {
                            mouse.mouse_move_down(enigo::MouseButton::Left);
                        }
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
