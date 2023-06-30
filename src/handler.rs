use crate::mouse::{Mouse, MouseRead};
use futures_util::StreamExt;
use warp::ws::WebSocket;

pub async fn room(ws: WebSocket, mut mouse: Mouse) {
    let (mut _tx, mut rx) = ws.split();

    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error: {}", e);
                break;
            }
        };

        if msg.is_close() {
            println!("closing socket");
            break;
        }
        match serde_json::from_str::<MouseRead>(msg.to_str().unwrap()) {
            Ok(read) => mouse.driver.mouse_move_relative(
                read.x_read * mouse.x_sen as i32,
                read.y_read * mouse.y_sen as i32,
            ),
            Err(err) => {
                eprintln!("websocket error: {}", err);
                break;
            }
        }
    }
}
