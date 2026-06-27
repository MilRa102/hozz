use anyhow::Context;
use std::path::Path;

/// A static cell that holds the global Sled database instance.
///
/// This module uses a `OnceCell` to ensure that the database is initialized only once
/// and shared across all parts of the application. The database is opened with specific
/// configuration options, including a custom cache capacity defined in the application's
/// configuration file.
static POOL: tokio::sync::OnceCell<sled::Db> = tokio::sync::OnceCell::const_new();

/// A singleton struct representing the global Sled database instance.
///
/// This struct provides access to the initialized Sled database through its `global()` method.
/// It encapsulates the initialization logic and ensures that the database is only created
/// once, even if multiple threads attempt to initialize it concurrently.
pub struct Database;

impl Database {
    /// Initializes the global Sled database instance.
    ///
    /// This method opens a new Sled database at the specified path with a configured cache capacity.
    /// The database is then stored in the static `POOL` cell, making it accessible globally via
    /// the `global()` method. If initialization fails (e.g., due to disk errors or permission issues),
    /// an error is logged and returned.
    ///
    /// # Arguments
    /// * `path` - The path to the directory where the Sled database files will be stored.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the database is initialized, or an error if failed.
    pub fn init(path: impl AsRef<Path>) -> anyhow::Result<()> {
        let db = sled::Config::new()
            .path(path)
            .cache_capacity(config::CONF.app.cache_capacity)
            .open()
            .context("Failed to open Sled Database")?;

        if let Err(e) = POOL.set(db) {
            tracing::error!(error = %e, "Failed to initialize database");
        };
        Ok(())
    }

    /// Returns a reference to the global Sled database instance.
    ///
    /// This method retrieves the singleton `sled::Db` instance that was initialized via the
    /// `init()` method. If the database has not been initialized yet, this will panic with
    /// a message indicating that the database is unavailable.
    ///
    /// # Returns
    /// * `&'static sled::Db` - A reference to the global Sled database.
    pub(crate) fn global() -> &'static sled::Db {
        POOL.get()
            .expect("The database was not initialized!")
    }
}
