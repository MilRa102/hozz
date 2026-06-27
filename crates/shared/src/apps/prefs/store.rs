pub struct PrefsStore;

impl db::SledManager<prefs::AppPrefs> for PrefsStore {
    const TREE_NAME: &'static str = "app_prefs";
}
