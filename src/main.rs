pub mod discovery;
pub mod handler;
pub mod mouse;

use crate::mouse::Mouse;

use handler::multiple;
use local_ip_address::local_ip;
use log::info;
use std::net::SocketAddr;
use warp::Filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let bind_address: SocketAddr = match std::env::var("BROADCAST_BIND_ADDRESS") {
        Ok(value) => value.parse::<SocketAddr>().unwrap(),
        Err(_) => {
            panic!("BROADCAST_BIND_ADDRESS env not found")
        }
    };

    let my_local_ip = match local_ip()? {
        std::net::IpAddr::V4(ip) => ip,
        _ => {
            panic!("IpV4 not found")
        }
    };

    info!("local ip: {:?}", my_local_ip);

    discovery::init_broadcast(bind_address).await;

    let mouse_filter = warp::any().map(|| Mouse::default());

    let multiple_route = warp::path("multiple")
        .and(warp::ws())
        .and(mouse_filter)
        .map(|ws: warp::ws::Ws, mouse| ws.on_upgrade(|socket| multiple(socket, mouse)));

    warp::serve(multiple_route).run((my_local_ip, 3030)).await;

    Ok(())
}
