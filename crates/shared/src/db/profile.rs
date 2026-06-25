use crate::{
    apps::{Profile, proxy::profile::ProfileV1},
    db::SledManager,
};

pub struct ProfileStore;

impl SledManager for ProfileStore {
    type Item = Profile;
    const TREE_NAME: &'static str = "profiles";

    fn decode(bytes: &[u8]) -> anyhow::Result<Self::Item> {
        if let Ok(profile) = bincode::deserialize::<Profile>(bytes) {
            return Ok(profile);
        }

        let old_profile = bincode::deserialize::<ProfileV1>(bytes)?;

        Ok(Profile {
            id: old_profile.id,
            name: None,
            source: old_profile.source,
            update_interval: old_profile.update_interval,
            enabled: true,
        })
    }
}

impl ProfileStore {
    pub fn add(&self, profile: &Profile) -> anyhow::Result<()> {
        self.save(&profile.id, profile)
    }
}
