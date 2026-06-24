use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecretType {
    Folder,
    Key,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretItem {
    pub name: String,
    pub path: String,
    pub secret_type: SecretType,
}

impl SecretItem {
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
