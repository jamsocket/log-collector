use crate::types::{LogMessage, LogMessageKind};
use anyhow::Result;
use bollard::{
    container::{LogOutput, LogsOptions},
    system::EventsOptions,
    Docker,
};
use dashmap::DashMap;
use futures_util::{StreamExt, TryStreamExt};
use tokio::{sync::mpsc::Sender, task::JoinHandle};

const PLANE_BACKEND_LABEL: &str = "dev.plane.backend";

pub async fn subscribe_to_container_logs(
    docker: Docker,
    backend_id: String,
    container_id: String,
    sender: Sender<LogMessage>,
) {
    tracing::info!(?container_id, backend_id, "Subscribing to container logs");

    let mut stream = docker.logs::<&str>(
        &container_id,
        Some(LogsOptions {
            stderr: true,
            stdout: true,
            follow: true,
            ..LogsOptions::default()
        }),
    );

    loop {
        let result = stream.next().await;

        let Some(log) = result else {
            tracing::info!("Log stream ended.");
            break;
        };

        let log = match log {
            Ok(log) => log,
            Err(e) => {
                tracing::error!("Error while streaming logs: {}", e);
                break;
            }
        };

        let log_payload = match &log {
            LogOutput::Console { message } => message,
            LogOutput::StdErr { message } => message,
            LogOutput::StdIn { message } => message,
            LogOutput::StdOut { message } => message,
        };

        let text = match std::str::from_utf8(log_payload) {
            Ok(log) => log.to_string(),
            Err(e) => {
                tracing::warn!(?e, "Error parsing log as UTF-8");
                continue;
            }
        };

        let kind = match log {
            LogOutput::StdOut { .. } => LogMessageKind::Stdout,
            LogOutput::StdErr { .. } => LogMessageKind::Stderr,
            _ => LogMessageKind::Stdout,
        };

        let message = LogMessage {
            backend_id: backend_id.clone(),
            kind,
            text,
        };

        if let Err(err) = sender.send(message).await {
            tracing::error!(?err, "Error sending message.");
        }
    }

    tracing::info!(?container_id, backend_id, "Log stream ended.");
}

pub async fn log_subscriber(sender: Sender<LogMessage>) -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;
    let log_task_map: DashMap<String, JoinHandle<()>> = DashMap::new();

    let mut watch_handle = docker.events(Some(EventsOptions {
        filters: vec![("type", vec!["container"]), ("event", vec!["start"])]
            .into_iter()
            .collect(),
        ..Default::default()
    }));

    // First, start logger for existing containers.
    let existing_containers = docker.list_containers::<&str>(None).await?;
    for container in existing_containers {
        let Some(labels) = container.labels else {
            continue;
        };

        if !labels.contains_key(PLANE_BACKEND_LABEL) {
            // Not a Plane backend.
            continue;
        };

        let Some(names) = container.names else {
            // No names
            continue;
        };

        // The '/' is not a typo -- this is a Docker thing.
        let Some(backend) = names.iter().find_map(|name| name.strip_prefix("/plane-")) else {
            // No plane backend name
            tracing::warn!(?names, "Encountered container with no plane backend name.");
            continue;
        };

        let Some(container_id) = container.id else {
            // No container ID?
            tracing::warn!("Encountered container without an ID?");
            continue;
        };

        log_task_map.entry(container_id.clone()).or_insert_with(|| {
            tokio::spawn(subscribe_to_container_logs(
                docker.clone(),
                backend.to_string(),
                container_id,
                sender.clone(),
            ))
        });
    }

    // Then, watch for containers to start and start loggers.
    while let Some(event) = watch_handle.try_next().await? {
        let Some(actor) = event.actor else {
            tracing::info!(?event, "Event did not have actor.");
            continue;
        };

        let Some(attributes) = actor.attributes else {
            tracing::info!(?actor, "Event did not have attributes.");
            continue;
        };

        if !attributes.contains_key(PLANE_BACKEND_LABEL) {
            tracing::info!(?attributes, "Event did not have plane backend label.");
            continue;
        };

        let Some(name) = attributes.get("name") else {
            tracing::info!(?attributes, "Event did not have name attribute.");
            continue;
        };

        let Some(backend) = name.strip_prefix("plane-") else {
            tracing::info!(?name, "Event did not have valid backend name.");
            continue;
        };

        let Some(container_id) = actor.id else {
            tracing::info!("Event did not have container id.");
            continue;
        };

        tracing::info!(container_id, backend, "Container started.");

        log_task_map.entry(container_id.clone()).or_insert_with(|| {
            tokio::spawn(subscribe_to_container_logs(
                docker.clone(),
                backend.to_string(),
                container_id,
                sender.clone(),
            ))
        });
    }

    Ok(())
}
