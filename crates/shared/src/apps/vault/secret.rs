//! Secret management types for the vault system.
//!
//! This module provides data structures and traits for representing and creating
//! secrets (files or folders) within the vault storage system.

use serde::{Deserialize, Serialize};

/// Represents the type of a secret item.
///
/// Determines whether the secret is a folder or a key (file).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecretType {
    /// A folder secret.
    Folder,
    /// A key (file) secret.
    Key,
}

/// Represents a single secret item in the vault.
///
/// Contains metadata about a secret including its name, path, and type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretItem {
    /// The display name of the secret.
    pub name: String,
    /// The storage path of the secret.
    pub path: String,
    /// The type of the secret (folder or key).
    pub secret_type: SecretType,
}

impl SecretItem {
    /// Creates a new folder secret item.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the folder.
    ///
    /// # Returns
    ///
    /// A new `SecretItem` configured as a folder.
    pub fn new_folder(name: String) -> Self {
        let path = name.clone();
        Self {
            name,
            path,
            secret_type: SecretType::Folder,
        }
    }
}

impl From<String> for SecretItem {
    /// Converts a string into a `SecretItem`.
    ///
    /// Automatically determines the type based on whether the string ends with '/'.
    ///
    /// # Arguments
    ///
    /// * `value` - The string representation of the secret.
    ///
    /// # Returns
    ///
    /// A new `SecretItem` with inferred type and normalized path.
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
