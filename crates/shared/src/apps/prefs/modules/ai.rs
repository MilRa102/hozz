use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, PrefsManager};

const PROVIDER_OPTIONS: &[&str] = &["gemini", "copilot", "ollama"];

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
            description: "Имя модели для выбранного AI-провайдера",
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
