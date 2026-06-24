#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    System,
    Network,
    Vault,
    Modules,
    Advanced,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::System => "Система",
            Category::Network => "Сеть",
            Category::Vault => "Хранилище",
            Category::Advanced => "Продвинутые",
            Category::Modules => "Модули",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    Restart,
    CoreReload,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingType {
    Toggle,
    TextInput,
    NumberInput { min: i32, max: i32 },
    Select(&'static [&'static str]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
    pub category: Category,
    pub setting_type: SettingType,
    pub requirements: &'static [Requirement],
    pub default_value: &'static str,
}
