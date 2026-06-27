/// Decodes a byte slice into a generic type `T`.
///
/// This function deserializes binary data using the `bincode` crate and converts it into
/// the specified type `T`. The type `T` must implement both `DeserializeOwned` and `Clone`.
/// If deserialization fails, it returns an error wrapped in `anyhow::Error`.
///
/// # Arguments
/// * `bytes` - A byte slice containing the serialized data.
/// * `T` - The target type to deserialize into.
///
/// # Returns
/// * `Result<T>` - Success if deserialization is successful, or an error if failed.
pub fn decode<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned + Clone,
{
    bincode::deserialize(bytes).map_err(Into::into)
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
pub trait SledManager<I>
where
    I: serde::Serialize + serde::de::DeserializeOwned + Clone,
{
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
    fn decode(bytes: &[u8]) -> anyhow::Result<I> {
        decode(bytes)
    }

    /// Retrieves all items from the Sled tree, filtering out any errors during decoding.
    ///
    /// This method iterates over all key-value pairs in the Sled tree, attempts to decode
    /// each value into an instance of type `I`, and collects them into a vector. Any values
    /// that fail to decode are filtered out.
    ///
    /// # Returns
    /// * `Result<Vec<I>>` - Success if all items are retrieved, or an error if the tree cannot be accessed.
    fn all(&self) -> anyhow::Result<Vec<I>> {
        let tree = self.tree()?;
        Ok(tree
            .iter()
            .values()
            .filter_map(std::result::Result::ok)
            .filter_map(|b| Self::decode(&b).ok())
            .collect())
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
    fn save(&self, key: &str, item: &I) -> anyhow::Result<()> {
        let tree = self.tree()?;
        let value = bincode::serialize(item)?;
        tree.insert(key, value)?;
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
    fn get(&self, key: &str) -> anyhow::Result<Option<I>> {
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
}
