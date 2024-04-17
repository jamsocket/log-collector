use crate::types::LogMessage;
use clap::Parser;
use docker::log_subscriber;
use init_tracing::init_tracing;
use sink::forward_to_socket;
use tokio::sync::mpsc::channel;

mod docker;
mod init_tracing;
mod sink;
mod types;

#[derive(Parser)]
struct Opts {
    url: String,
}

#[tokio::main]
async fn main() {
    init_tracing();
    let opts = Opts::parse();

    let (send_log_message, recv_log_message) = channel::<LogMessage>(1024);

    tokio::spawn(forward_to_socket(opts.url, recv_log_message));

    log_subscriber(send_log_message).await.unwrap();
}
