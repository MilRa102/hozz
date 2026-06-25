use crate::apps::{Orchestrator, alert::types::Alert};

/// Trait for logging events through the orchestrator's state notification system.
///
/// Implementations of this trait allow components to report status updates,
/// errors, warnings, and informational messages by converting them into `Alert` objects.
pub trait LoggingLayer {
    /// Reports an error message by creating an `Alert::error`.
    ///
    /// # Arguments
    /// * `message` - The error message to log.
    fn error(&self, message: impl Into<String>);

    /// Reports an informational message by creating an `Alert::info`.
    ///
    /// # Arguments
    /// * `message` - The informational message to log.
    fn info(&self, message: impl Into<String>);

    /// Reports a successful status by creating an `Alert::ok`.
    ///
    /// # Arguments
    /// * `message` - The success message to log.
    fn ok(&self, message: impl Into<String>);

    /// Reports a warning message by creating an `Alert::warning`.
    ///
    /// # Arguments
    /// * `message` - The warning message to log.
    fn warning(&self, message: impl Into<String>);
}

/// Implementation of the `LoggingLayer` trait for the `Orchestrator` type.
///
/// This implementation delegates all logging operations to the orchestrator's state,
/// which converts the messages into appropriate `Alert` objects and notifies listeners.
impl LoggingLayer for Orchestrator {
    /// Reports an error message by creating an `Alert::error`.
    ///
    /// # Arguments
    /// * `message` - The error message to log.
    fn error(&self, message: impl Into<String>) {
        self.state.notify(Alert::error(message));
    }

    /// Reports an informational message by creating an `Alert::info`.
    ///
    /// # Arguments
    /// * `message` - The informational message to log.
    fn info(&self, message: impl Into<String>) {
        self.state.notify(Alert::info(message));
    }

    /// Reports a successful status by creating an `Alert::ok`.
    ///
    /// # Arguments
    /// * `message` - The success message to log.
    fn ok(&self, message: impl Into<String>) {
        self.state.notify(Alert::ok(message));
    }

    /// Reports a warning message by creating an `Alert::warning`.
    ///
    /// # Arguments
    /// * `message` - The warning message to log.
    fn warning(&self, message: impl Into<String>) {
        self.state.notify(Alert::warning(message));
    }
}
