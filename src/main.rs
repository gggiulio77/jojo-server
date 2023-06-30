pub mod handler;
pub mod mouse;

use crate::mouse::Mouse;

use handler::room;
use warp::Filter;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let mouse_filter = warp::any().map(|| Mouse::default());

    let routes = warp::path("room")
        .and(warp::ws())
        .and(mouse_filter)
        .map(|ws: warp::ws::Ws, mouse| ws.on_upgrade(|socket| room(socket, mouse)));

    warp::serve(routes).run(([192, 168, 0, 163], 3030)).await;
}
