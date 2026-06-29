use std::str::FromStr;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Default, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub enum Target {
    App,
    #[default]
    Host,
}

#[derive(Debug, Default, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub enum Direction {
    Direct,
    Reject,
    #[default]
    Auto,
}

#[derive(Debug, Default, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub target: Target,
    pub direction: Direction,
    pub is_active: bool,
    pub is_ignored: bool,
    pub amt: u16,
}

#[derive(Debug, Default, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct RuleBuilder {
    pub name: String,
    pub target: Target,
    pub direction: Direction,
    pub is_active: bool,
    pub is_ignored: bool,
    pub amt: u16,
}

impl AsRef<str> for Target {
    fn as_ref(&self) -> &str {
        match self {
            Self::App => "PROCESS-NAME",
            Self::Host => "DOMAIN-SUFFIX",
        }
    }
}

impl AsRef<str> for Direction {
    fn as_ref(&self) -> &str {
        match self {
            Self::Direct => "DIRECT",
            Self::Reject => "REJECT",
            Self::Auto => "AUTO",
        }
    }
}

impl FromStr for Direction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "DIRECT" => Self::Direct,
            "REJECT" => Self::Reject,
            _ => Self::Auto,
        })
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} ({}) -> {} [{}]",
            if self.is_active { "✓" } else { "✗" },
            self.name,
            self.target.as_ref(),
            self.direction.as_ref(),
            self.amt
        )
    }
}
