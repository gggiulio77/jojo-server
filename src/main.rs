use futures_util::{FutureExt, StreamExt};
use warp::{ws::WebSocket, Filter};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let routes = warp::path("room")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(move |socket| handler(socket)));

    warp::serve(routes).run(([192, 168, 0, 163], 3030)).await;
}

async fn handler(ws: WebSocket) {
    let (mut _tx, mut rx) = ws.split();

    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error: {}", e);
                break;
            }
        };

        println!("[msg]: {:?}", msg.to_str().unwrap());
    }
}
