use db::SledManager;
use prefs::AppPrefs;

use crate::{memory::MemoryPolicyKind, model::ProviderKind};

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
    pub const KEY_MODEL_GEMINI: &'static str = "ai.model.gemini";
    pub const KEY_MODEL_COPILOT: &'static str = "ai.model.copilot";
    pub const KEY_MODEL_OLLAMA: &'static str = "ai.model.ollama";
    pub const KEY_GEMINI_API_KEY: &'static str = "ai.api_key.gemini";
    pub const KEY_COPILOT_API_KEY: &'static str = "ai.api_key.copilot";
    pub const KEY_TAVILY_API_KEY: &'static str = "ai.api_key.tavily";
    pub const KEY_OLLAMA_BASE_URL: &'static str = "ai.ollama.base_url";
    pub const KEY_MEMORY_POLICY: &'static str = "ai.memory.policy";
    pub const KEY_MEMORY_MAX_TOKENS: &'static str = "ai.memory.max_tokens";
    pub const KEY_MEMORY_MAX_MESSAGES: &'static str = "ai.memory.max_messages";
    pub const KEY_MEMORY_MAP_ENABLED: &'static str = "ai.memory_map.enabled";
    pub const KEY_MEMORY_MAP_PROVIDER: &'static str = "ai.memory_map.embedding.provider";
    pub const KEY_MEMORY_MAP_MODEL: &'static str = "ai.memory_map.embedding.model";

    fn value(&self, key: &str) -> Option<String> {
        SledManager::get(self, key)
            .ok()
            .flatten()
            .map(|pref: AppPrefs| pref.value)
    }

    pub fn is_enabled(&self) -> bool {
        self.value(Self::KEY_ENABLED)
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn provider(&self) -> Option<ProviderKind> {
        self.value(Self::KEY_PROVIDER)
            .and_then(|v| v.parse().ok())
    }

    pub fn model(&self) -> Option<String> {
        self.value(Self::KEY_MODEL)
    }

    pub fn provider_model(&self, provider: ProviderKind) -> Option<String> {
        let key = match provider {
            ProviderKind::Gemini => Self::KEY_MODEL_GEMINI,
            ProviderKind::Copilot => Self::KEY_MODEL_COPILOT,
            ProviderKind::Ollama => Self::KEY_MODEL_OLLAMA,
        };

        self.value(key).or_else(|| self.model())
    }

    pub fn effective_provider(&self, fallback: ProviderKind) -> ProviderKind {
        self.provider().unwrap_or(fallback)
    }

    pub fn effective_model(
        &self,
        provider: ProviderKind,
        fallback_model: Option<&str>,
    ) -> String {
        self.provider_model(provider).unwrap_or_else(|| {
            fallback_model
                .map(str::to_string)
                .unwrap_or_else(|| match provider {
                    ProviderKind::Gemini => "gemini-2.5-flash".to_string(),
                    ProviderKind::Copilot => "gpt-5.3-codex".to_string(),
                    ProviderKind::Ollama => "llama3".to_string(),
                })
        })
    }

    pub fn gemini_api_key(&self) -> Option<String> {
        self.value(Self::KEY_GEMINI_API_KEY)
    }

    pub fn copilot_api_key(&self) -> Option<String> {
        self.value(Self::KEY_COPILOT_API_KEY)
    }

    pub fn tavily_api_key(&self) -> Option<String> {
        self.value(Self::KEY_TAVILY_API_KEY)
            .filter(|value| !value.trim().is_empty())
    }

    pub fn ollama_base_url(&self) -> String {
        self.value(Self::KEY_OLLAMA_BASE_URL)
            .unwrap_or_else(|| "http://localhost:11434".to_string())
    }

    pub fn memory_policy(&self) -> MemoryPolicyKind {
        self.value(Self::KEY_MEMORY_POLICY)
            .and_then(|v| v.parse().ok())
            .unwrap_or_default()
    }

    /// Token budget for the memory window. A missing, unparseable or zero value
    /// falls back to the default rather than truncating the history to nothing.
    pub fn memory_max_tokens(&self) -> usize {
        self.value(Self::KEY_MEMORY_MAX_TOKENS)
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|budget| *budget > 0)
            .unwrap_or(crate::memory::DEFAULT_MAX_TOKENS)
    }

    /// Message budget for the sliding window, with the same zero-guard as
    /// [`Self::memory_max_tokens`].
    pub fn memory_max_messages(&self) -> usize {
        self.value(Self::KEY_MEMORY_MAX_MESSAGES)
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .unwrap_or(crate::memory::DEFAULT_MAX_MESSAGES)
    }

    pub fn memory_map_enabled(&self) -> bool {
        self.value(Self::KEY_MEMORY_MAP_ENABLED)
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn memory_map_provider(&self) -> Option<ProviderKind> {
        self.value(Self::KEY_MEMORY_MAP_PROVIDER)
            .and_then(|v| v.parse().ok())
            .or(self.provider())
    }

    pub fn memory_map_model(&self, provider: ProviderKind) -> String {
        self.value(Self::KEY_MEMORY_MAP_MODEL)
            .or_else(|| self.provider_model(provider))
            .unwrap_or_else(|| match provider {
                ProviderKind::Gemini => "gemini-embedding-001".to_string(),
                ProviderKind::Copilot => "text-embedding-3-small".to_string(),
                ProviderKind::Ollama => "all-minilm".to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::test_support::init_db;

    static PREF_LOCK: Mutex<()> = Mutex::new(());

    #[allow(clippy::unwrap_used)]
    fn set_pref(key: &str, value: &str) {
        SledManager::save(&AiPrefsReader, key, &AppPrefs::new(key, value)).unwrap();
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn reads_provider_and_model_from_shared_app_prefs_tree() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_PROVIDER, "ollama");
        set_pref(AiPrefsReader::KEY_MODEL, "llama3");

        let reader = AiPrefsReader;
        assert_eq!(reader.provider(), Some(ProviderKind::Ollama));
        assert_eq!(reader.model().as_deref(), Some("llama3"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn is_enabled_reflects_stored_bool() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_ENABLED, "true");
        assert!(AiPrefsReader.is_enabled());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn memory_map_toggle_reads_stored_bool() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_MEMORY_MAP_ENABLED, "true");
        assert!(AiPrefsReader.memory_map_enabled());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn memory_map_provider_and_model_use_override_keys_when_present() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_PROVIDER, "gemini");
        set_pref(AiPrefsReader::KEY_MEMORY_MAP_PROVIDER, "ollama");
        set_pref(AiPrefsReader::KEY_MEMORY_MAP_MODEL, "all-minilm");

        let reader = AiPrefsReader;
        assert_eq!(reader.memory_map_provider(), Some(ProviderKind::Ollama));
        assert_eq!(reader.memory_map_model(ProviderKind::Ollama), "all-minilm");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn ollama_base_url_falls_back_to_default_when_unset() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        assert_eq!(
            AiPrefsReader.ollama_base_url(),
            "http://localhost:11434"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn provider_specific_model_keys_are_preferred_over_legacy_key() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_MODEL, "legacy-model");
        set_pref(AiPrefsReader::KEY_MODEL_GEMINI, "gemini-2.5-pro");

        let reader = AiPrefsReader;
        assert_eq!(
            reader
                .provider_model(ProviderKind::Gemini)
                .as_deref(),
            Some("gemini-2.5-pro")
        );
        assert_eq!(
            reader.effective_model(ProviderKind::Gemini, Some("fallback-model")),
            "gemini-2.5-pro"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn legacy_model_is_used_as_fallback_when_targeted_key_is_missing() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_MODEL, "legacy-model");

        let reader = AiPrefsReader;
        assert_eq!(
            reader
                .provider_model(ProviderKind::Copilot)
                .as_deref(),
            Some("legacy-model")
        );
        assert_eq!(
            reader.effective_model(ProviderKind::Copilot, Some("fallback-model")),
            "legacy-model"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn effective_model_falls_back_to_provider_default_when_unset() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();

        let reader = AiPrefsReader;
        assert_eq!(
            reader.effective_model(ProviderKind::Ollama, None),
            "llama3"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn tavily_api_key_roundtrips_when_present() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_TAVILY_API_KEY, "tvly-test-key");

        let reader = AiPrefsReader;
        assert_eq!(
            reader.tavily_api_key().as_deref(),
            Some("tvly-test-key")
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn tavily_api_key_is_none_when_blank() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        set_pref(AiPrefsReader::KEY_TAVILY_API_KEY, "   ");

        assert_eq!(AiPrefsReader.tavily_api_key(), None);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn provider_is_none_when_key_missing() {
        let _guard = PREF_LOCK.lock().unwrap();
        init_db();
        assert_eq!(
            AiPrefsReader.value("ai.unset.provider.test"),
            None
        );
    }
}
