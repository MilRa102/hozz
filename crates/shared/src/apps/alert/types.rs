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
    pub fn message(&self) -> &str {
        match self {
            Self::Info { msg, .. }
            | Self::Error { msg, .. }
            | Self::Ok { msg, .. }
            | Self::Warning { msg, .. } => msg,
        }
    }
}
