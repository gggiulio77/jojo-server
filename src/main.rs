use futures_util::future::join;
use jojo_server::initialize;
use log::*;
use std::net::Ipv4Addr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let (tx, rx) = crossbeam_channel::unbounded::<jojo_common::room::RoomEvent>();
    let ip_local = Ipv4Addr::new(192, 168, 0, 163);
    let port = 3000;

    let task = initialize(ip_local, port, tx);

    let server = tokio::spawn(async { task.await });

    let listener = tokio::spawn(async move {
        info!("LISTENING");

        while let Ok(event) = rx.recv() {
            info!("EVENT EMITTED: {:?}", event)
        }
    });

    let (_, _) = join(server, listener).await;

    Ok(())
}
