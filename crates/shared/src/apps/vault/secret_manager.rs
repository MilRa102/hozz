use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Map, Value as Json};
use vaultrs::{
    api::{ResponseWrapper, WrappedResponse, kv2::requests::ReadSecretRequest},
    client::{Client, VaultClient, VaultClientSettingsBuilder},
    kv2,
};

use crate::apps::{
    LoggingLayer, Orchestrator,
    vault::{SecretItem, SecretVisit, TokenInfo},
};

pub type SecretData = Map<String, Json>;
pub type SecretWrappedResponse = WrappedResponse<ReadSecretRequest>;

/// A trait for managing secrets through Vault's KV backend.
///
/// This trait provides an interface for authenticating with Vault, listing and reading secrets,
/// inspecting metadata, wrapping and unwrapping secret payloads, and tracking frequently used
/// paths. Implementations are expected to provide access to the underlying Vault client and the
/// local state used for visit history.
///
/// # Methods
/// * `vault()` - Builds a `VaultClient` from the configured Vault address and token.
/// * `session()` - Returns the current authentication token information.
/// * `secrets()` - Lists secret paths under a mount and prefix.
/// * `secret()` - Reads a secret value from the KV store.
/// * `secret_meta()` - Reads metadata for a secret, such as versions and timestamps.
/// * `secret_wrap()` - Wraps a secret read response in a single-use wrap token.
/// * `secret_wrap_lookup()` - Looks up information for an existing wrap token.
/// * `mounts()` - Lists available KV mount points.
/// * `track_visit()` - Records a secret access event for usage history.
/// * `frequent_visits()` - Returns the most frequently visited secrets.
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
    /// * `Result<SecretData>` - A JSON map of key-value pairs on success, or an error if reading fails.
    async fn secret(&self, mount: &str, path: &str) -> Result<SecretData>;

    /// Reads metadata for a specific secret.
    ///
    /// The metadata includes information such as the current version, creation time, and deletion
    /// time when available. The provided path is normalized by trimming leading slashes before
    /// the request is sent.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point (for example, `secret`).
    /// * `path` - The path to the secret.
    ///
    /// # Returns
    /// * `Result<SecretData>` - A JSON object containing the secret metadata.
    async fn secret_meta(&self, mount: &str, path: &str) -> Result<SecretData>;

    /// Wraps a secret read response in a single-use wrap token.
    ///
    /// This method reads the secret at the provided mount and path and returns the Vault response
    /// wrapper that can later be used to look up or unwrap the wrapped payload. The path is
    /// normalized by trimming leading slashes before the request is sent.
    ///
    /// # Arguments
    /// * `mount` - The Vault mount point that contains the secret.
    /// * `path` - The path to the secret within the mount.
    ///
    /// # Returns
    /// * `Result<SecretWrappedResponse>` - A wrap token response that can be inspected or unwrapped.
    async fn secret_wrap(&self, mount: &str, path: &str)
    -> Result<SecretWrappedResponse>;

    /// Looks up information for an existing wrap token.
    ///
    /// This method queries Vault for the state of a previously issued wrap token without
    /// consuming it. It is useful for inspecting the wrap token metadata or verifying that it is
    /// still valid.
    ///
    /// # Arguments
    /// * `token` - The wrap token to look up.
    async fn secret_wrap_lookup(&self, token: &str) -> Result<()>;

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
    /// * `Result<SecretData>` - A JSON map of key-value pairs on success, or an error if reading fails.
    async fn secret(&self, mount: &str, path: &str) -> Result<SecretData> {
        let client = self.vault()?;
        let path = path.trim_start_matches('/');
        let secret = kv2::read::<SecretData>(&client, mount, path).await?;
        Ok(secret)
    }

    /// Reads metadata for a specific secret.
    ///
    /// The metadata is fetched from Vault and converted into a JSON object so it can be handled
    /// consistently by the rest of the application. Leading slashes are trimmed from the path
    /// before the request is issued.
    async fn secret_meta(&self, mount: &str, path: &str) -> Result<SecretData> {
        let client = self.vault()?;
        let path = path.trim_start_matches('/');

        let metadata = kv2::read_metadata(&client, mount, path).await?;
        let json = serde_json::to_value(metadata)?;

        if let Json::Object(map) = json {
            Ok(map)
        } else {
            Err(anyhow!("Unexpected metadata format"))
        }
    }

    /// Wraps a secret read response in a single-use wrap token.
    ///
    /// The request is built from the provided mount and path, and the resulting Vault wrap
    /// response is returned so it can be inspected or unwrapped later.
    async fn secret_wrap(
        &self,
        mount: &str,
        path: &str,
    ) -> Result<SecretWrappedResponse> {
        let client = self.vault()?;
        let path = path.trim_start_matches('/');

        let endpoint = ReadSecretRequest::builder()
            .mount(mount)
            .path(path)
            .build()
            .map_err(|e| anyhow!("Failed to build read secret request: {}", e))?;

        let wrap_response = endpoint.wrap(&client).await?;
        Ok(wrap_response)
    }

    /// Looks up information for an existing wrap token.
    ///
    /// The wrap response is queried against Vault without consuming the token so callers can
    /// inspect its metadata or verify that it is still valid.
    async fn secret_wrap_lookup(&self, token: &str) -> Result<()> {
        let client = self.vault()?;
        vaultrs::sys::wrapping::lookup(&client, token).await?;
        Ok(())
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
