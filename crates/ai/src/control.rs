use futures::future::AbortHandle;
use strum::{Display, EnumString};

/// A small command enum for controlling an in-flight streaming generation.
/// The current provider layer only needs a single cancellation action, so we
/// keep the contract intentionally narrow and explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum StreamCommand {
    #[strum(serialize = "cancel")]
    Cancel,
}

/// Type-erased stream-control handle for an in-flight generation, so callers
/// (the engine, the UI) do not need to know the concrete provider response
/// type `R` behind the running stream.
#[async_trait::async_trait]
pub trait StreamControl: Send + Sync {
    async fn apply(&self, command: StreamCommand);
}

pub(crate) struct ResponseControl(pub AbortHandle);

#[async_trait::async_trait]
impl StreamControl for ResponseControl {
    async fn apply(&self, command: StreamCommand) {
        match command {
            StreamCommand::Cancel => self.0.abort(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::StreamCommand;

    #[test]
    fn stream_command_round_trips_with_strum() {
        let command = StreamCommand::from_str("cancel").expect("cancel should parse");
        assert_eq!(command, StreamCommand::Cancel);
        assert_eq!(command.to_string(), "cancel");
    }
}
