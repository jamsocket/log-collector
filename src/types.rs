use serde::{Deserialize, Serialize};

// Note: this should remain JSON-equivalent to Plane 0.3.0 log messages, described here:
// https://github.com/drifting-in-space/plane/blob/6ab01c6c1ea02471fb1ed112cfced4b34189fbb8/core/src/messages/agent.rs#L51-L64

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum LogMessageKind {
    Stdout,
    Stderr,
    Meta,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogMessage {
    pub backend_id: String,
    pub kind: LogMessageKind,
    pub text: String,
}
