use rkyv::{
    Archive, Deserialize, Serialize,
    api::high::{HighDeserializer, HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::Error,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
};

/// Decodes an unaligned byte slice from Sled into a generic type `T`.
///
/// This implementation uses `rkyv 0.8`. Since Sled's `IVec` is not guaranteed
/// to be memory-aligned, we use `rkyv::util::AlignedVec` to safely copy
/// the bytes before accessing the archived root.
pub fn decode<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: Archive,
    T::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
        + Deserialize<T, HighDeserializer<Error>>,
{
    let mut aligned_bytes = AlignedVec::<16>::with_capacity(bytes.len());
    aligned_bytes.extend_from_slice(bytes);

    let archived = rkyv::access::<T::Archived, Error>(&aligned_bytes)
        .map_err(|e| anyhow::anyhow!("Rkyv validation failed: {e}"))?;

    let item: T = rkyv::deserialize::<T, Error>(archived)
        .map_err(|e| anyhow::anyhow!("Rkyv deserialization failed: {e}"))?;

    Ok(item)
}

/// A trait defining the interface for managing data storage using Sled.
///
/// This trait provides methods to perform CRUD operations on a Sled database tree.
/// It includes functionality for retrieving all items, saving individual items, fetching
/// specific items by key, and deleting items. The `SledManager` struct implements this
/// trait to manage the lifecycle of data stored in Sled databases.
///
/// # Generic Parameter
/// * `I` - The type of items being managed. Must implement `Serialize`, `DeserializeOwned`, and `Clone`.
///
/// # Methods
/// * `TREE_NAME` - A constant defining the name of the Sled tree to use.
/// * `db()` - Returns a reference to the global Sled database instance.
/// * `tree()` - Opens and returns a reference to the specified Sled tree.
/// * `decode()` - Decodes binary data into the item type `I`.
/// * `all()` - Retrieves all items from the tree, filtering out any errors during decoding.
/// * `save()` - Saves an item to the database with the given key.
/// * `get()` - Retrieves a specific item from the database by its key.
/// * `delete()` - Removes an item from the database by its key.
pub trait SledManager<I> {
    /// The name of the Sled tree to use for this manager.
    const TREE_NAME: &'static str;

    /// Returns a reference to the global Sled database instance.
    ///
    /// This method provides access to the singleton `sled::Db` instance that has been
    /// initialized and stored in the `Database` module. If the database has not been
    /// initialized yet, this will panic.
    ///
    /// # Returns
    /// * `&'static sled::Db` - A reference to the global Sled database.
    fn db(&self) -> &'static sled::Db {
        crate::Database::global()
    }

    /// Opens and returns a reference to the specified Sled tree.
    ///
    /// This method opens the Sled tree identified by `Self::TREE_NAME` and returns a
    /// reference to it. If the tree cannot be opened (e.g., due to missing files or
    /// permission issues), it returns an error.
    ///
    /// # Returns
    /// * `Result<sled::Tree>` - Success if the tree is opened, or an error if failed.
    fn tree(&self) -> anyhow::Result<sled::Tree> {
        self.db()
            .open_tree(Self::TREE_NAME)
            .map_err(anyhow::Error::msg)
    }

    /// Decodes binary data into the item type `I`.
    ///
    /// This is a wrapper around the `decode` function that uses `Self::decode` to deserialize
    /// binary data into an instance of type `I`.
    ///
    /// # Arguments
    /// * `bytes` - A byte slice containing the serialized data.
    ///
    /// # Returns
    /// * `Result<I>` - Success if deserialization is successful, or an error if failed.
    fn decode(bytes: &[u8]) -> anyhow::Result<I>
    where
        I: Archive,
        I::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
            + Deserialize<I, HighDeserializer<Error>>,
    {
        decode(bytes)
    }

    /// Retrieves all items from the Sled tree, logging and skipping any entries
    /// that fail to read or decode instead of silently discarding them.
    ///
    /// # Returns
    /// * `Result<Vec<I>>` - Success if all items are retrieved, or an error if the tree cannot be accessed.
    fn all(&self) -> anyhow::Result<Vec<I>>
    where
        I: Archive,
        I::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
            + Deserialize<I, HighDeserializer<Error>>,
    {
        let tree = self.tree()?;
        let mut items = Vec::new();
        for entry in tree.iter().values() {
            match entry {
                Ok(bytes) => match Self::decode(&bytes) {
                    Ok(item) => items.push(item),
                    Err(error) => {
                        tracing::warn!(tree = %Self::TREE_NAME, %error, "Failed to decode item from Sled tree");
                    },
                },
                Err(error) => {
                    tracing::warn!(tree = %Self::TREE_NAME, %error, "Failed to read entry from Sled tree");
                },
            }
        }
        Ok(items)
    }

    /// Saves an item to the database with the given key.
    ///
    /// This method serializes the provided item into binary format using `bincode` and stores
    /// it in the Sled tree under the specified key. The changes are immediately flushed to disk.
    ///
    /// # Arguments
    /// * `key` - The unique identifier for the item being saved.
    /// * `item` - The item to be saved to the database.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the item is saved, or an error if failed.
    fn save(&self, key: &str, item: &I) -> anyhow::Result<()>
    where
        I: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
    {
        let tree = self.tree()?;
        let value = rkyv::to_bytes::<Error>(item)
            .map_err(|e| anyhow::anyhow!("Rkyv serialization failed: {e}"))?;
        tree.insert(key, value.into_vec())?;
        tree.flush()?;
        Ok(())
    }

    /// Retrieves a specific item from the database by its key.
    ///
    /// This method fetches the binary data associated with the given key from the Sled tree,
    /// decodes it into an instance of type `I`, and returns it wrapped in `Some`. If no item
    /// is found or if decoding fails, it returns `None`.
    ///
    /// # Arguments
    /// * `key` - The unique identifier of the item to retrieve.
    ///
    /// # Returns
    /// * `Result<Option<I>>` - Success if the item is retrieved, or an error if failed.
    fn get(&self, key: &str) -> anyhow::Result<Option<I>>
    where
        I: Archive,
        I::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
            + Deserialize<I, HighDeserializer<Error>>,
    {
        let tree = self.tree()?;
        if let Some(bytes) = tree.get(key)? {
            let item = Self::decode(&bytes)?;
            return Ok(Some(item));
        }
        Ok(None)
    }

    /// Removes an item from the database by its key.
    ///
    /// This method deletes the entry associated with the given key from the Sled tree.
    /// If the key does not exist, it simply returns without error.
    ///
    /// # Arguments
    /// * `key` - The unique identifier of the item to delete.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the item is deleted, or an error if failed.
    fn delete(&self, key: &str) -> anyhow::Result<()> {
        let tree = self.tree()?;
        tree.remove(key)?;
        Ok(())
    }

    /// Retrieves all items whose key starts with the given prefix, logging and
    /// skipping any entries that fail to read or decode.
    ///
    /// Sled iterates keys in lexicographic byte order, so this is useful for
    /// namespaced/composite keys (e.g. `"<conversation_id>:<timestamp>"`) where a
    /// zero-padded timestamp suffix keeps results chronologically ordered without
    /// needing to decode the whole tree.
    ///
    /// # Arguments
    /// * `prefix` - The key prefix to scan for.
    ///
    /// # Returns
    /// * `Result<Vec<I>>` - Success if the tree is scanned, or an error if failed.
    fn scan_prefix(&self, prefix: &str) -> anyhow::Result<Vec<I>>
    where
        I: Archive,
        I::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>
            + Deserialize<I, HighDeserializer<Error>>,
    {
        let tree = self.tree()?;
        let mut items = Vec::new();
        for entry in tree.scan_prefix(prefix.as_bytes()).values() {
            match entry {
                Ok(bytes) => match Self::decode(&bytes) {
                    Ok(item) => items.push(item),
                    Err(error) => {
                        tracing::warn!(tree = %Self::TREE_NAME, %prefix, %error, "Failed to decode item from Sled tree");
                    },
                },
                Err(error) => {
                    tracing::warn!(tree = %Self::TREE_NAME, %prefix, %error, "Failed to read entry from Sled tree");
                },
            }
        }
        Ok(items)
    }

    /// Removes all items whose key starts with the given prefix, atomically.
    ///
    /// All matching removals are collected into a single `sled::Batch` and applied
    /// in one atomic operation, so a failure or crash midway can't leave the
    /// prefix partially deleted.
    ///
    /// # Arguments
    /// * `prefix` - The key prefix to scan and delete.
    ///
    /// # Returns
    /// * `Result<()>` - Success if all matching items are removed atomically, or an error if failed.
    fn delete_prefix(&self, prefix: &str) -> anyhow::Result<()> {
        let tree = self.tree()?;
        let mut batch = sled::Batch::default();
        let mut removed = 0usize;
        for key in tree.scan_prefix(prefix.as_bytes()).keys() {
            match key {
                Ok(key) => {
                    batch.remove(key);
                    removed += 1;
                },
                Err(error) => {
                    tracing::warn!(tree = %Self::TREE_NAME, %prefix, %error, "Failed to read key while scanning prefix for deletion");
                },
            }
        }
        tree.apply_batch(batch)?;
        tree.flush()?;
        tracing::debug!(tree = %Self::TREE_NAME, %prefix, removed, "Deleted keys by prefix");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use rkyv::{Archive, Deserialize, Serialize};

    use super::SledManager;

    #[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
    struct Item {
        value: u32,
    }

    struct ItemStore;

    impl SledManager<Item> for ItemStore {
        const TREE_NAME: &'static str = "test_items";
    }

    static INIT: Once = Once::new();

    /// Initializes a process-wide temporary Sled database for tests. Safe to call
    /// from every test — only the first call actually opens the database.
    fn init_db() {
        INIT.call_once(|| {
            let path = std::env::temp_dir()
                .join(format!("hozz-db-tests-{}", std::process::id()));
            crate::Database::init(path).expect("failed to init test database");
        });
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn save_get_delete_roundtrip() {
        init_db();
        let store = ItemStore;
        store
            .save("roundtrip", &Item { value: 1 })
            .unwrap();
        assert_eq!(
            store.get("roundtrip").unwrap(),
            Some(Item { value: 1 })
        );
        store.delete("roundtrip").unwrap();
        assert_eq!(store.get("roundtrip").unwrap(), None);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn get_returns_none_for_missing_key() {
        init_db();
        let store = ItemStore;
        assert_eq!(store.get("does-not-exist").unwrap(), None);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn scan_prefix_orders_by_key_and_ignores_other_prefixes() {
        init_db();
        let store = ItemStore;
        store
            .save("scan:001", &Item { value: 1 })
            .unwrap();
        store
            .save("scan:002", &Item { value: 2 })
            .unwrap();
        store
            .save("other:001", &Item { value: 99 })
            .unwrap();

        let items = store.scan_prefix("scan:").unwrap();
        assert_eq!(items, vec![Item { value: 1 }, Item { value: 2 }]);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn delete_prefix_removes_all_matching_atomically() {
        init_db();
        let store = ItemStore;
        store
            .save("bulk:001", &Item { value: 1 })
            .unwrap();
        store
            .save("bulk:002", &Item { value: 2 })
            .unwrap();
        store
            .save("bulk-keep:001", &Item { value: 3 })
            .unwrap();

        store.delete_prefix("bulk:").unwrap();

        assert!(store.scan_prefix("bulk:").unwrap().is_empty());
        assert_eq!(store.scan_prefix("bulk-keep:").unwrap().len(), 1);
    }
}
