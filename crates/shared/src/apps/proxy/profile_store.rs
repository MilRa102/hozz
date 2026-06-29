use db::SledManager;

use crate::apps::Profile;

pub struct ProfileStore;

impl SledManager<Profile> for ProfileStore {
    const TREE_NAME: &'static str = "profiles";
}

impl ProfileStore {
    pub fn add(&self, profile: &Profile) -> anyhow::Result<()> {
        self.save(&profile.id, profile)
    }
}
