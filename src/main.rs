use futures_util::future::join;
use log::*;
use mouse_server::{initialize, room};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let (tx, rx) = crossbeam_channel::unbounded::<room::RoomEvent>();

    let task = initialize(tx);

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
