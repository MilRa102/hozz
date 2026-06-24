use crate::app::{alert::Alert, orchestrator::Orchestrator};

pub trait LoggingLayer {
    fn error(&self, message: impl Into<String>);
    fn info(&self, message: impl Into<String>);
    fn ok(&self, message: impl Into<String>);
    fn warning(&self, message: impl Into<String>);
}

impl LoggingLayer for Orchestrator {
    fn error(&self, message: impl Into<String>) {
        self.state.notify(Alert::error(message));
    }

    fn info(&self, message: impl Into<String>) {
        self.state.notify(Alert::info(message));
    }

    fn ok(&self, message: impl Into<String>) {
        self.state.notify(Alert::ok(message));
    }

    fn warning(&self, message: impl Into<String>) {
        self.state.notify(Alert::warning(message));
    }
}
