use serde::{Deserialize, Serialize};
use vaultrs::api::token::responses::LookupTokenResponse;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    pub id: String,
    pub display_name: String,
    pub policies: Vec<String>,
    pub ttl: u64,
    pub creation_ttl: u64,
    pub renewable: Option<bool>,
    pub expire_time: Option<String>,
}

impl TokenInfo {
    pub fn policies_str(&self) -> String {
        self.policies.join(",")
    }
}

impl From<LookupTokenResponse> for TokenInfo {
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
