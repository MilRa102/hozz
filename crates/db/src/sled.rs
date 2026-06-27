pub fn decode<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned + Clone,
{
    bincode::deserialize(bytes).map_err(Into::into)
}

pub trait SledManager<I>
where
    I: serde::Serialize + serde::de::DeserializeOwned + Clone,
{
    const TREE_NAME: &'static str;

    fn db(&self) -> &'static sled::Db {
        crate::Database::global()
    }

    fn tree(&self) -> anyhow::Result<sled::Tree> {
        self.db()
            .open_tree(Self::TREE_NAME)
            .map_err(anyhow::Error::msg)
    }

    fn decode(bytes: &[u8]) -> anyhow::Result<I> {
        decode(bytes)
    }

    fn all(&self) -> anyhow::Result<Vec<I>> {
        let tree = self.tree()?;
        Ok(tree
            .iter()
            .values()
            .filter_map(std::result::Result::ok)
            .filter_map(|b| Self::decode(&b).ok())
            .collect())
    }

    fn save(&self, key: &str, item: &I) -> anyhow::Result<()> {
        let tree = self.tree()?;
        let value = bincode::serialize(item)?;
        tree.insert(key, value)?;
        tree.flush()?;
        Ok(())
    }

    fn get(&self, key: &str) -> anyhow::Result<Option<I>> {
        let tree = self.tree()?;
        if let Some(bytes) = tree.get(key)? {
            let item = Self::decode(&bytes)?;
            return Ok(Some(item));
        }
        Ok(None)
    }

    fn delete(&self, key: &str) -> anyhow::Result<()> {
        let tree = self.tree()?;
        tree.remove(key)?;
        Ok(())
    }
}
