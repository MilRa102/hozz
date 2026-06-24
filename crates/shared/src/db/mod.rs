pub mod app;
pub(crate) mod prefs;
pub mod profile;
pub mod rule;
pub mod vault;

use std::path::Path;

use anyhow::{Context, Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use sled::{Config, Db};
use tokio::sync::OnceCell;

pub static DB: OnceCell<Db> = OnceCell::const_new();

pub struct Database;

impl Database {
    pub fn init(path: impl AsRef<Path>) -> Result<()> {
        let db = Config::new()
            .path(path)
            .cache_capacity(10 * 1024 * 1024)
            .open()
            .context("Failed to open Sled database")?;

        let _ = DB.set(db);
        Ok(())
    }

    pub fn global() -> &'static Db {
        DB.get()
            .expect("The database was not initialized!")
    }
}

pub trait SledManager {
    type Item: Serialize + DeserializeOwned + Clone;

    const TREE_NAME: &'static str;

    fn db(&self) -> &'static Db {
        Database::global()
    }

    fn tree(&self) -> Result<sled::Tree> {
        self.db()
            .open_tree(Self::TREE_NAME)
            .map_err(Error::msg)
    }

    fn decode(bytes: &[u8]) -> Result<Self::Item> {
        bincode::deserialize(bytes).map_err(Into::into)
    }

    fn all(&self) -> Result<Vec<Self::Item>> {
        let items = self
            .tree()?
            .iter()
            .values()
            .filter_map(std::result::Result::ok)
            .filter_map(|b| Self::decode(&b).ok())
            .collect();
        Ok(items)
    }

    fn save(&self, key: &str, item: &Self::Item) -> Result<()> {
        let tree = self.tree()?;
        let bytes = bincode::serialize(item)?;

        tree.insert(key, bytes)?;
        tree.flush()?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<Self::Item>> {
        if let Some(bytes) = self.tree()?.get(key)? {
            let item = Self::decode(&bytes)?;
            return Ok(Some(item));
        }
        Ok(None)
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.tree()?.remove(key)?;
        Ok(())
    }
}
