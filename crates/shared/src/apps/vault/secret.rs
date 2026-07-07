//! Secret management types for the vault system.
//!
//! This module provides data structures and traits for representing and creating
//! secrets (files or folders) within the vault storage system.

use serde::{Deserialize, Serialize};

/// Describes whether a Vault secret entry represents a folder or a single value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecretType {
    /// A folder entry.
    Folder,
    /// A secret value entry.
    Key,
}

/// A single secret entry discovered under a Vault mount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretItem {
    /// The display name of the secret entry.
    pub name: String,
    /// The storage path of the entry in Vault.
    pub path: String,
    /// The entry type: folder or key.
    pub secret_type: SecretType,
}

impl SecretItem {
    /// Creates a folder item with the provided name.
    pub fn new_folder(name: String) -> Self {
        let path = name.clone();
        Self {
            name,
            path,
            secret_type: SecretType::Folder,
        }
    }
}

/// The payload returned by Vault when a secret is read or unwrapped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretObject {
    /// The secret data values.
    pub data: serde_json::Map<String, serde_json::Value>,
    /// The metadata associated with the secret.
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl From<String> for SecretItem {
    /// Converts a Vault listing entry into a `SecretItem`.
    ///
    /// The type is inferred from whether the entry ends with `/`.
    fn from(value: String) -> Self {
        let is_folder = value.ends_with('/');
        Self {
            name: value.trim_end_matches('/').to_string(),
            path: value,
            secret_type: if is_folder {
                SecretType::Folder
            } else {
                SecretType::Key
            },
        }
    }
}
