use db::{SledManager, decode};

use crate::apps::{Profile, proxy::profile::ProfileV1};

pub struct ProfileStore;

impl SledManager<Profile> for ProfileStore {
    const TREE_NAME: &'static str = "profiles";

    fn decode(bytes: &[u8]) -> anyhow::Result<Profile> {
        if let Ok(profile) = decode::<Profile>(bytes) {
            return Ok(profile);
        }

        let old_profile = decode::<ProfileV1>(bytes)?;

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
