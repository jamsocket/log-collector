use crate::types::LogMessage;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::{select, sync::mpsc::Receiver};
use tokio_tungstenite::tungstenite::Message;

enum Impossible {}

async fn run_socket(url: &str, receiver: &mut Receiver<LogMessage>) -> Result<Impossible> {
    let (mut conn, _) = tokio_tungstenite::connect_async(url).await?;

    loop {
        select! {
            msg = receiver.recv() => {
                if let Some(msg) = msg {
                    let msg_str = serde_json::to_string(&msg)?;
                    let msg = Message::Text(msg_str);
                    let _ = conn.send(msg).await;
                } else {
                    return Err(anyhow::anyhow!("Receiver closed"));
                }
            },
            incoming = conn.next() => {
                if incoming.is_none() {
                    return Err(anyhow::anyhow!("Connection closed"));
                }
            }
        }
    }
}

pub async fn forward_to_socket(url: String, mut receiver: Receiver<LogMessage>) {
    loop {
        tracing::info!("Attempting socket connection.");
        if let Err(err) = run_socket(&url, &mut receiver).await {
            tracing::warn!("Error: {}", err);
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
