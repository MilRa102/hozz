use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Alert {
    Info { id: Uuid, msg: String },
    Error { id: Uuid, msg: String },
    Ok { id: Uuid, msg: String },
    Warning { id: Uuid, msg: String },
}

impl Alert {
    /// Create a message with the status 'Info'
    ///
    /// # Arguments
    /// * `msg` - Message body
    ///
    /// # Returns
    /// Message object
    ///
    /// # Example
    /// ```
    /// Alert::info("This is an info message");
    /// ```
    pub fn info(msg: impl Into<String>) -> Self {
        Self::Info {
            id: Uuid::new_v4(),
            msg: msg.into(),
        }
    }

    /// Create a message with the status 'Error'
    ///
    /// # Arguments
    /// * `msg` - Message body
    ///
    /// # Returns
    /// Message object
    ///
    /// # Example
    /// ```
    /// Alert::error("Unidentified error");
    /// ```
    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error {
            id: Uuid::new_v4(),
            msg: msg.into(),
        }
    }

    /// Create a message with the status 'Warning'
    ///
    /// # Arguments
    /// * `msg` - Message body
    ///
    /// # Returns
    /// Message object
    ///
    /// # Example
    /// ```
    /// Alert::warning("Something is wrong");
    /// ```
    pub fn warning(msg: impl Into<String>) -> Self {
        Self::Warning {
            id: Uuid::new_v4(),
            msg: msg.into(),
        }
    }

    /// Create a message with the status 'Ok'
    ///
    /// # Arguments
    /// * `msg` - Message body
    ///
    /// # Returns
    /// Message object
    ///
    /// # Example
    /// ```
    /// Alert::ok("Everything is fine");
    /// ```
    pub fn ok(msg: impl Into<String>) -> Self {
        Self::Ok {
            id: Uuid::new_v4(),
            msg: msg.into(),
        }
    }

    /// Get ID regardless of type
    ///
    /// # Returns
    /// Object ID
    ///
    /// # Example
    /// ```
    /// Alert::ok("Everything is fine").id();
    /// ```
    #[must_use]
    pub fn id(&self) -> Uuid {
        match self {
            Self::Info { id, .. }
            | Self::Error { id, .. }
            | Self::Ok { id, .. }
            | Self::Warning { id, .. } => *id,
        }
    }

    /// Receive messages
    ///
    /// # Returns
    /// Receive messages
    ///
    /// # Example
    /// ```
    /// Alert::ok("Everything is fine").message();
    /// ```
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Info { msg, .. }
            | Self::Error { msg, .. }
            | Self::Ok { msg, .. }
            | Self::Warning { msg, .. } => msg,
        }
    }
}
