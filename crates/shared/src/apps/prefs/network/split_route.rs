use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, PrefsManager};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum SplitRouteRules {
    #[default]
    Ru,
    Eu,
}

impl FromStr for SplitRouteRules {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s {
            "Европа" => Self::Eu,
            _ => Self::Ru,
        };
        Ok(result)
    }
}

impl AsRef<str> for SplitRouteRules {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ru => "Россия",
            Self::Eu => "Европа",
        }
    }
}

impl SplitRouteRules {
    pub const SPEC: &'static [&'static str] = &["Россия", "Европа"];

    pub(crate) fn match_rule() -> Vec<String> {
        vec!["MATCH,AUTO".to_string()]
    }

    pub(crate) fn as_rules(&self) -> Vec<String> {
        let mut rules = vec![
            "IP-CIDR,127.0.0.1/8,DIRECT,no-resolve",
            "IP-CIDR,192.168.0.0/16,DIRECT,no-resolve",
            "IP-CIDR,10.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,172.16.0.0/12,DIRECT,no-resolve",
        ];
        let custom_rules = match self {
            Self::Ru => vec![
                "GEOIP,RU,DIRECT",
                "GEOSITE,yandex,DIRECT",
                "GEOSITE,mailru,DIRECT",
                "GEOSITE,google,AUTO",
                "GEOSITE,azure,AUTO",
                "GEOSITE,microsoft,AUTO",
                "GEOSITE,github,AUTO",
                "GEOSITE,youtube,AUTO",
                "GEOSITE,telegram,AUTO",
                "GEOSITE,twitter,AUTO",
                "GEOSITE,netflix,AUTO",
                "GEOSITE,spotify,AUTO",
            ],
            Self::Eu => vec![
                "GEOSITE,google,DIRECT",
                "GEOSITE,azure,DIRECT",
                "GEOSITE,microsoft,DIRECT",
                "GEOSITE,github,DIRECT",
                "GEOSITE,youtube,DIRECT",
                "GEOSITE,telegram,DIRECT",
                "GEOSITE,twitter,DIRECT",
                "GEOSITE,netflix,DIRECT",
                "GEOSITE,spotify,DIRECT",
            ],
        };
        rules.extend(custom_rules);
        rules
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>()
    }
}

pub(crate) struct SplitRouteCapability;

impl PreferenceKey for SplitRouteCapability {
    const ID: &'static str = "network.split_routing";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for SplitRouteCapability {
    fn meta(&self) -> prefs::SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Пакет роутинга",
            description: "Определяет какой список изначальных правил установлен в приложении.",
            tags: &["split", "routing", "маршрут", "раздельно"],
            category: Category::Network,
            setting_type: SettingType::Select(SplitRouteRules::SPEC),
            requirements: &[Requirement::Restart],
            default_value: SplitRouteRules::Ru.as_ref(),
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(orch.get_origin(Self::ID).map(|p| p.value))
    }

    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        orch.warning("Для применения изменений перезагрузите приложение");
        Ok(())
    }
}
