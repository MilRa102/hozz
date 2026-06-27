use anyhow::Context;

static POOL: tokio::sync::OnceCell<sled::Db> = tokio::sync::OnceCell::const_new();

pub struct Database;

impl Database {
    pub fn init(path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let db = sled::Config::new()
            .path(path)
            .cache_capacity(10 * 1024 * 1024)
            .open()
            .context("Failed to open Sled Database")?;

        if let Err(e) = POOL.set(db) {
            tracing::error!(error = %e, "Failed to initialize database");
        };
        Ok(())
    }

    pub(crate) fn global() -> &'static sled::Db {
        POOL.get()
            .expect("The database was not initialized!")
    }
}
