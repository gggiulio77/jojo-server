pub mod handler;
pub mod mouse;

use crate::mouse::Mouse;

use handler::{multiple, single};
use warp::Filter;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let mouse_filter = warp::any().map(|| Mouse::default());

    let single_route = warp::path("single")
        .and(warp::ws())
        .and(mouse_filter)
        .map(|ws: warp::ws::Ws, mouse| ws.on_upgrade(|socket| single(socket, mouse)));

    let multiple_route = warp::path("multiple")
        .and(warp::ws())
        .and(mouse_filter)
        .map(|ws: warp::ws::Ws, mouse| ws.on_upgrade(|socket| multiple(socket, mouse)));

    warp::serve(single_route.or(multiple_route))
        .run(([192, 168, 0, 163], 3030))
        .await;
}
