use db::SledManager;
use prefs::AppPrefs;

use crate::model::ProviderKind;

/// Reads AI-related settings directly from the same Sled tree used by
/// `shared::apps::prefs::store::PrefsStore` (`TREE_NAME = "app_prefs"`), without the
/// `ai` crate needing to depend on `shared`. The corresponding `PreferenceHook`
/// implementations that write these keys (and expose them in the Settings UI) live
/// in `shared` — the key names below must match those exactly.
pub struct AiPrefsReader;

impl SledManager<AppPrefs> for AiPrefsReader {
    const TREE_NAME: &'static str = "app_prefs";
}

impl AiPrefsReader {
    pub const KEY_ENABLED: &'static str = "module.ai";
    pub const KEY_PROVIDER: &'static str = "ai.provider";
    pub const KEY_MODEL: &'static str = "ai.model";
    pub const KEY_GEMINI_API_KEY: &'static str = "ai.api_key.gemini";
    pub const KEY_COPILOT_API_KEY: &'static str = "ai.api_key.copilot";
    pub const KEY_OLLAMA_BASE_URL: &'static str = "ai.ollama.base_url";

    fn value(&self, key: &str) -> Option<String> {
        SledManager::get(self, key).ok().flatten().map(|pref: AppPrefs| pref.value)
    }

    pub fn is_enabled(&self) -> bool {
        self.value(Self::KEY_ENABLED)
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn provider(&self) -> Option<ProviderKind> {
        self.value(Self::KEY_PROVIDER).and_then(|v| v.parse().ok())
    }

    pub fn model(&self) -> Option<String> {
        self.value(Self::KEY_MODEL)
    }

    pub fn gemini_api_key(&self) -> Option<String> {
        self.value(Self::KEY_GEMINI_API_KEY)
    }

    pub fn copilot_api_key(&self) -> Option<String> {
        self.value(Self::KEY_COPILOT_API_KEY)
    }

    pub fn ollama_base_url(&self) -> String {
        self.value(Self::KEY_OLLAMA_BASE_URL)
            .unwrap_or_else(|| "http://localhost:11434".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::init_db;

    #[allow(clippy::unwrap_used)]
    fn set_pref(key: &str, value: &str) {
        SledManager::save(&AiPrefsReader, key, &AppPrefs::new(key, value)).unwrap();
    }

    #[test]
    fn reads_provider_and_model_from_shared_app_prefs_tree() {
        init_db();
        set_pref(AiPrefsReader::KEY_PROVIDER, "ollama");
        set_pref(AiPrefsReader::KEY_MODEL, "llama3");

        let reader = AiPrefsReader;
        assert_eq!(reader.provider(), Some(ProviderKind::Ollama));
        assert_eq!(reader.model().as_deref(), Some("llama3"));
    }

    #[test]
    fn is_enabled_reflects_stored_bool() {
        init_db();
        set_pref(AiPrefsReader::KEY_ENABLED, "true");
        assert!(AiPrefsReader.is_enabled());
    }

    #[test]
    fn ollama_base_url_falls_back_to_default_when_unset() {
        init_db();
        assert_eq!(AiPrefsReader.ollama_base_url(), "http://localhost:11434");
    }

    #[test]
    fn provider_is_none_when_key_missing() {
        init_db();
        assert_eq!(AiPrefsReader.value("ai.unset.provider.test"), None);
    }
}
