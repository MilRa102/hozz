use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Map, Value as Json};
use vaultrs::{
    client::{Client, VaultClient, VaultClientSettingsBuilder},
    kv2,
};

use crate::apps::{
    LoggingLayer, Orchestrator,
    vault::{SecretItem, SecretVisit, TokenInfo},
};

/// A trait defining the interface for managing secrets in a Vault system.
///
/// This trait provides methods to interact with Vault's KV (Key-Value) store,
/// including retrieving sessions, listing and reading secrets, and tracking access patterns.
/// It is implemented by the `Orchestrator` struct to provide centralized secret management.
///
/// # Methods
/// * `vault()` - Creates a new `VaultClient` configured with the current vault settings.
/// * `session()` - Retrieves the current authentication token and session information.
/// * `secrets()` - Lists all secrets within a specific mount point and path.
/// * `secret()` - Reads the content of a specific secret from the KV store.
/// * `mounts()` - Returns a list of all configured KV mount points in the Vault system.
/// * `track_visit()` - Records an access event for a specific secret path to track usage patterns.
/// * `frequent_visits()` - Returns a sorted list of secrets accessed most frequently.
#[async_trait]
pub trait SecretManager {
    /// Creates a new `VaultClient` instance configured with the current vault settings.
    ///
    /// This method fetches the vault configuration from the local store and constructs
    /// a client using the provided address and token. It returns a result indicating
    /// success or failure in creating the client.
    ///
    /// # Returns
    /// * `Result<VaultClient>` - The configured Vault client on success, or an error if configuration is missing.
    fn vault(&self) -> Result<VaultClient>;

    /// Retrieves the current authentication session information.
    ///
    /// This method authenticates with the Vault server and returns the token info,
    /// which includes the session ID and token itself. It ensures the client is properly
    /// authenticated before returning the session details.
    ///
    /// # Returns
    /// * `Result<TokenInfo>` - The token information on success, or an error if authentication fails.
    async fn session(&self) -> Result<TokenInfo>;

    /// Lists all secrets within a specific mount point and path.
    ///
    /// This method queries the KV store for all keys matching the provided mount and path.
    /// It returns a vector of `SecretItem` structs, each representing a secret key found.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point prefix (e.g., "secret").
    /// * `path` - The specific path within the mount to query (e.g., "user/keys").
    ///
    /// # Returns
    /// * `Result<Vec<SecretItem>>` - A list of secret items on success, or an error if the query fails.
    async fn secrets(&self, mount: &str, path: &str) -> Result<Vec<SecretItem>>;

    /// Reads the content of a specific secret from the KV store.
    ///
    /// This method retrieves the JSON map containing the values for a given secret path.
    /// It trims leading slashes from the path to ensure consistent key resolution.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point prefix.
    /// * `path` - The specific path within the mount to read.
    ///
    /// # Returns
    /// * `Result<Map<String, Json>>` - A JSON map of key-value pairs on success, or an error if reading fails.
    async fn secret(&self, mount: &str, path: &str) -> Result<Map<String, Json>>;

    /// Returns a list of all configured KV mount points in the Vault system.
    ///
    /// This method queries the Vault system for all registered mounts and filters them
    /// to include only those of type "kv" (Key-Value). It returns the paths of these mounts.
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - A list of mount point paths on success, or an error if listing fails.
    async fn mounts(&self) -> Result<Vec<String>>;

    /// Records an access event for a specific secret path to track usage patterns.
    ///
    /// This method increments the visit count for a given mount and path combination in
    /// the local vault configuration store. If the entry already exists, it updates the count;
    /// otherwise, it creates a new entry. It also maintains a limit of 100 entries, keeping
    /// only the top 50 most visited secrets to optimize storage.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point prefix.
    /// * `path` - The specific path within the mount being accessed.
    fn track_visit(&self, mount: &str, path: &str);

    /// Returns a sorted list of secrets accessed most frequently.
    ///
    /// This method retrieves the visit history from the local configuration and returns
    /// the top 10 most visited secrets, sorted in descending order by access count.
    ///
    /// # Returns
    /// * `Vec<SecretVisit>` - A vector of secret visits sorted by frequency on success, or an empty vector if no data is available.
    fn frequent_visits(&self) -> Vec<SecretVisit>;
}

#[async_trait]
impl SecretManager for Orchestrator {
    /// Creates a new `VaultClient` instance configured with the current vault settings.
    ///
    /// This method fetches the vault configuration (URL and token) from the local `AppStore`.
    /// It constructs a `VaultClientSettingsBuilder`, configures it with the retrieved values,
    /// and creates a new client. If the configuration is missing or invalid, it returns an error.
    ///
    /// # Returns
    /// * `Result<VaultClient>` - The configured Vault client on success, or an error if configuration is missing.
    fn vault(&self) -> Result<VaultClient> {
        let cfg = self
            .vaults
            .fetch()
            .ok_or(anyhow!("Vault client not initialized"))?;

        Ok(VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(cfg.url)
                .token(cfg.token)
                .build()?,
        )?)
    }

    /// Retrieves the current authentication session information.
    ///
    /// This method creates a `VaultClient` instance and immediately queries it for the current
    /// session data using the `lookup()` method. The result is converted into a `TokenInfo`
    /// struct which contains the necessary token details.
    ///
    /// # Returns
    /// * `Result<TokenInfo>` - The token information on success, or an error if authentication fails.
    async fn session(&self) -> Result<TokenInfo> {
        let client = self.vault()?;
        Ok(client.lookup().await?.into())
    }

    /// Lists all secrets within a specific mount point and path.
    ///
    /// This method creates a `VaultClient` instance and uses the `kv2::list` function to
    /// retrieve all keys matching the provided mount and path. The resulting list of keys
    /// is then mapped into `SecretItem` structs for further processing or display.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point prefix (e.g., "secret").
    /// * `path` - The specific path within the mount to query (e.g., "user/keys").
    ///
    /// # Returns
    /// * `Result<Vec<SecretItem>>` - A list of secret items on success, or an error if the query fails.
    async fn secrets(&self, mount: &str, path: &str) -> Result<Vec<SecretItem>> {
        let client = self.vault()?;
        let keys = kv2::list(&client, mount, path).await?;
        let items = keys.into_iter().map(SecretItem::from).collect();
        Ok(items)
    }

    /// Reads the content of a specific secret from the KV store.
    ///
    /// This method creates a `VaultClient` instance and uses `kv2::read` to fetch the
    /// JSON map associated with the given path. It trims any leading slashes from the path
    /// to ensure consistent key resolution across different Vault configurations.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point prefix.
    /// * `path` - The specific path within the mount to read.
    ///
    /// # Returns
    /// * `Result<Map<String, Json>>` - A JSON map of key-value pairs on success, or an error if reading fails.
    async fn secret(&self, mount: &str, path: &str) -> Result<Map<String, Json>> {
        let client = self.vault()?;
        let path = path.trim_start_matches('/');
        let secret = kv2::read::<Map<String, Json>>(&client, mount, path).await?;
        Ok(secret)
    }

    /// Returns a list of all configured KV mount points in the Vault system.
    ///
    /// This method queries the Vault system using `vaultrs::sys::mount::list` to retrieve
    /// all registered mounts. It then filters this list to include only those with a mount
    /// type of "kv" (Key-Value). The paths are trimmed of trailing slashes and collected
    /// into a vector of strings.
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - A list of mount point paths on success, or an error if listing fails.
    async fn mounts(&self) -> Result<Vec<String>> {
        let client = self.vault()?;
        let mounts = vaultrs::sys::mount::list(&client).await?;

        let kvs = mounts
            .into_iter()
            .filter(|(_, info)| info.mount_type == "kv")
            .map(|(path, _)| path.trim_end_matches('/').to_string())
            .collect();
        Ok(kvs)
    }

    /// Records an access event for a specific secret path to track usage patterns.
    ///
    /// This method updates the local vault configuration store (`AppStore`) with visit statistics.
    /// It searches for an existing entry matching the provided mount and path; if found, it increments
    /// the visit count. If not found, it creates a new `SecretVisit` entry.
    ///
    /// To prevent unbounded growth of the configuration store, the method enforces a limit:
    /// if more than 100 entries exist, it sorts them by visit count (descending) and truncates
    /// the list to keep only the top 50 most visited secrets.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point prefix.
    /// * `path` - The specific path within the mount being accessed.
    fn track_visit(&self, mount: &str, path: &str) {
        if let Some(mut cfg) = self.vaults.fetch() {
            if let Some(visit) = cfg
                .visited
                .iter_mut()
                .find(|v| v.mount == mount && v.path == path)
            {
                visit.count = visit.count.saturating_add(1);
            } else {
                cfg.visited.push(SecretVisit::new(mount, path));
            }

            if cfg.visited.len() > 100 {
                cfg.visited
                    .sort_by_key(|b| std::cmp::Reverse(b.count));
                cfg.visited.truncate(50);
            }

            if let Err(e) = self.vaults.update(&cfg) {
                tracing::warn!(error = %e, "Failed to update visit information.");
                self.warning("Не удалось обновить информацию о посещении");
            }
        }
    }

    /// Returns a sorted list of secrets accessed most frequently.
    ///
    /// This method retrieves the visit history from the local configuration store and returns
    /// the top 10 most visited secrets, sorted in descending order by access count. If no data
    /// is available or an error occurs during fetch, it returns an empty vector.
    ///
    /// # Returns
    /// * `Vec<SecretVisit>` - A vector of secret visits sorted by frequency on success, or an empty vector if no data is available.
    fn frequent_visits(&self) -> Vec<SecretVisit> {
        self.vaults
            .fetch()
            .map(|cfg| {
                let mut visits = cfg.visited.clone();
                visits.sort_by_key(|b| std::cmp::Reverse(b.count));
                visits.into_iter().take(10).collect()
            })
            .unwrap_or_default()
    }
}
