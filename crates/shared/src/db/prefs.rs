pub struct PrefsStore;

impl crate::db::SledManager for PrefsStore {
    type Item = prefs::AppPrefs;
    const TREE_NAME: &'static str = "app_prefs";
}
