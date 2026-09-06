use std::sync::Arc;

use ai::{AiPrefsReader, ProviderKind};
use async_trait::async_trait;
use db::SledManager;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, PrefsManager};

const PROVIDER_OPTIONS: &[&str] = &["gemini", "copilot", "ollama"];
const MEMORY_POLICY_OPTIONS: &[&str] = &["none", "token"];
const MEMORY_MIN_TOKENS: i32 = 1_024;
const MEMORY_MAX_TOKENS: i32 = 1_000_000;

pub struct ChatCapability;

impl PreferenceKey for ChatCapability {
    const ID: &'static str = "module.ai";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for ChatCapability {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "AI Чат",
            description: "Отображать раздел AI-чата в приложении",
            tags: &["ai", "chat", "ассистент", "модуль"],
            category: Category::Modules,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::Restart],
            default_value: "false",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(Some(
            orch.preference_is_active::<Self>().to_string(),
        ))
    }

    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        orch.info("Для применения изменений, пожалуйста перезагрузите приложение");
        Ok(())
    }
}

pub struct AiProviderSetting;

impl PreferenceKey for AiProviderSetting {
    const ID: &'static str = "ai.provider";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiProviderSetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "AI Провайдер",
            description: "Провайдер модели для AI-чата",
            tags: &["ai", "provider", "gemini", "copilot", "ollama"],
            category: Category::Advanced,
            setting_type: SettingType::Select(PROVIDER_OPTIONS),
            requirements: &[],
            default_value: "gemini",
        }
    }

    async fn before_execute(
        &self,
        _orch: Arc<Orchestrator>,
        new: &str,
    ) -> anyhow::Result<()> {
        if !PROVIDER_OPTIONS.contains(&new) {
            anyhow::bail!("Неподдерживаемый AI провайдер: {new}");
        }
        Ok(())
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}

pub struct AiModelSetting;

impl PreferenceKey for AiModelSetting {
    const ID: &'static str = "ai.model";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiModelSetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "AI Модель",
            description: "Модель для выбранного AI-провайдера",
            tags: &["ai", "model", "llm"],
            category: Category::Advanced,
            setting_type: SettingType::TextInput,
            requirements: &[],
            default_value: "gemini-2.5-flash",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }

    async fn execute(&self, orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let provider = orch
            .get_origin(AiProviderSetting::ID)
            .and_then(|pref| pref.value.parse().ok())
            .unwrap_or(ProviderKind::Gemini);
        let key = match provider {
            ProviderKind::Gemini => AiPrefsReader::KEY_MODEL_GEMINI,
            ProviderKind::Copilot => AiPrefsReader::KEY_MODEL_COPILOT,
            ProviderKind::Ollama => AiPrefsReader::KEY_MODEL_OLLAMA,
        };

        orch.prefs
            .save(key, &prefs::AppPrefs::new(key, new))
    }
}

pub struct AiGeminiKeySetting;

impl PreferenceKey for AiGeminiKeySetting {
    const ID: &'static str = "ai.api_key.gemini";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiGeminiKeySetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Gemini API Key",
            description: "API-ключ Google Gemini",
            tags: &["ai", "gemini", "api", "key"],
            category: Category::Advanced,
            setting_type: SettingType::TextInput,
            requirements: &[],
            default_value: "",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}

pub struct AiCopilotKeySetting;

impl PreferenceKey for AiCopilotKeySetting {
    const ID: &'static str = "ai.api_key.copilot";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiCopilotKeySetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Copilot API Key",
            description: "API-ключ GitHub Copilot",
            tags: &["ai", "copilot", "api", "key"],
            category: Category::Advanced,
            setting_type: SettingType::TextInput,
            requirements: &[],
            default_value: "",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}

pub struct AiTavilyKeySetting;

impl PreferenceKey for AiTavilyKeySetting {
    const ID: &'static str = "ai.api_key.tavily";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiTavilyKeySetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Tavily API Key",
            description: "API-ключ Tavily для веб-поиска",
            tags: &["ai", "tavily", "api", "key", "search"],
            category: Category::Advanced,
            setting_type: SettingType::TextInput,
            requirements: &[Requirement::Restart],
            default_value: "",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}

pub struct AiOllamaUrlSetting;

impl PreferenceKey for AiOllamaUrlSetting {
    const ID: &'static str = "ai.ollama.base_url";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiOllamaUrlSetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Ollama Base URL",
            description: "Базовый URL Ollama (например http://localhost:11434)",
            tags: &["ai", "ollama", "url", "base_url"],
            category: Category::Advanced,
            setting_type: SettingType::TextInput,
            requirements: &[],
            default_value: "http://localhost:11434",
        }
    }

    async fn before_execute(
        &self,
        _orch: Arc<Orchestrator>,
        new: &str,
    ) -> anyhow::Result<()> {
        if !(new.starts_with("http://") || new.starts_with("https://")) {
            anyhow::bail!("Ollama URL должен начинаться с http:// или https://");
        }
        Ok(())
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}

pub struct AiMemoryPolicySetting;

impl PreferenceKey for AiMemoryPolicySetting {
    const ID: &'static str = "ai.memory.policy";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiMemoryPolicySetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Политика памяти",
            description: "Как обрезать историю диалога перед отправкой модели: \
                          none — отправлять целиком, token — только последние \
                          сообщения в рамках бюджета токенов",
            tags: &["ai", "memory", "память", "context", "контекст"],
            category: Category::Advanced,
            setting_type: SettingType::Select(MEMORY_POLICY_OPTIONS),
            requirements: &[],
            default_value: "token",
        }
    }

    async fn before_execute(
        &self,
        _orch: Arc<Orchestrator>,
        new: &str,
    ) -> anyhow::Result<()> {
        if !MEMORY_POLICY_OPTIONS.contains(&new) {
            anyhow::bail!("Неподдерживаемая политика памяти: {new}");
        }
        Ok(())
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}

pub struct AiMemoryMaxTokensSetting;

impl PreferenceKey for AiMemoryMaxTokensSetting {
    const ID: &'static str = "ai.memory.max_tokens";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AiMemoryMaxTokensSetting {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Бюджет токенов памяти",
            description: "Сколько токенов истории отправлять модели при политике \
                          «token». Старые сообщения остаются в чате, но выпадают \
                          из контекста",
            tags: &["ai", "memory", "память", "tokens", "токены"],
            category: Category::Advanced,
            setting_type: SettingType::NumberInput {
                min: MEMORY_MIN_TOKENS,
                max: MEMORY_MAX_TOKENS,
            },
            requirements: &[],
            default_value: "32000",
        }
    }

    async fn before_execute(
        &self,
        _orch: Arc<Orchestrator>,
        new: &str,
    ) -> anyhow::Result<()> {
        let budget: i32 = new.trim().parse().map_err(|_| {
            anyhow::anyhow!("Бюджет токенов должен быть целым числом: {new}")
        })?;

        if !(MEMORY_MIN_TOKENS..=MEMORY_MAX_TOKENS).contains(&budget) {
            anyhow::bail!(
                "Бюджет токенов должен быть от {MEMORY_MIN_TOKENS} до {MEMORY_MAX_TOKENS}"
            );
        }
        Ok(())
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }
}
