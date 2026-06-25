//! Token information types for the vault system.
//!
//! This module provides data structures for representing token details
//! and converting between internal representations and API responses.

use serde::{Deserialize, Serialize};
use vaultrs::api::token::responses::LookupTokenResponse;

/// Represents detailed information about a token.
///
/// Contains metadata such as ID, display name, policies, TTLs, and expiration time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    /// The unique identifier of the token.
    pub id: String,
    /// The human-readable name of the token.
    pub display_name: String,
    /// List of policies associated with the token.
    pub policies: Vec<String>,
    /// Time-to-live in seconds for the token.
    pub ttl: u64,
    /// Time-to-live in seconds for token creation.
    pub creation_ttl: u64,
    /// Indicates whether the token can be renewed.
    pub renewable: Option<bool>,
    /// Optional expiration time as an ISO 8601 string.
    pub expire_time: Option<String>,
}

impl TokenInfo {
    /// Returns a comma-separated string of all policies associated with the token.
    ///
    /// # Returns
    ///
    /// A `String` containing all policies joined by commas.
    pub fn policies_str(&self) -> String {
        self.policies.join(",")
    }
}

impl From<LookupTokenResponse> for TokenInfo {
    /// Converts a `LookupTokenResponse` into a `TokenInfo`.
    ///
    /// This implementation maps all fields from the API response to the internal struct.
    ///
    /// # Arguments
    ///
    /// * `value` - The `LookupTokenResponse` from the vault API.
    ///
    /// # Returns
    ///
    /// A new `TokenInfo` instance populated with data from the response.
    fn from(value: LookupTokenResponse) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            policies: value.policies,
            ttl: value.ttl,
            creation_ttl: value.creation_ttl,
            renewable: value.renewable,
            expire_time: value.expire_time,
        }
    }
}
