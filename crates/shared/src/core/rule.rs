use crate::{
    core::models::rule::{Direction, Rule, RuleBuilder, Target},
    utils::generate_id,
};

impl Rule {
    /// Creates a new `RuleBuilder` with the given name.
    #[must_use]
    pub fn builder(name: &str) -> RuleBuilder {
        RuleBuilder::new(name)
    }

    /// Creates a new Rule with the given name and default
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Creates a new Rule with the given name and default
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Creates a new Rule with the given name and default
    pub fn ignore(&mut self) {
        self.is_ignored = true;
    }

    /// Toggles the active status of the rule.
    pub fn toggle_active(&mut self) {
        self.is_active = !self.is_active;
    }

    /// Toggles the ignore status of the rule.
    pub fn toggle_ignore(&mut self) {
        self.is_ignored = !self.is_ignored;
    }

    /// Increments the amount of times this rule has been matched.
    pub fn inc_amt(&mut self) {
        self.amt = self.amt.saturating_add(1);
    }

    /// Converts the Rule into a string format that can be used in the configuration file.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Checks if the rule is ignored.
    #[must_use]
    pub fn is_ignored(&self) -> bool {
        self.is_ignored
    }

    /// Checks if the rule is an app rule.
    #[must_use]
    pub fn is_app(&self) -> bool {
        matches!(self.target, Target::App)
    }

    /// Checks if the rule is a host rule.
    #[must_use]
    pub fn is_host(&self) -> bool {
        matches!(self.target, Target::Host)
    }

    /// Converts the Rule into a string format that can be used in the configuration file.
    #[must_use]
    pub fn to_rule(&self) -> String {
        format!(
            "{},{},{}",
            self.target.as_ref(),
            self.name,
            self.direction.as_ref()
        )
    }
}

impl RuleBuilder {
    /// Creates a new `RuleBuilder` with the given name.
    /// The target defaults to Host, the direction defaults to Auto, and the rule is active by default.
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Sets the target of the rule (App or Host).
    #[must_use]
    pub fn target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    /// Sets the direction of the rule (Direct, Reject, Proxy, Auto).
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets whether the rule is active or not.
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Sets whether the rule is ignored or not.
    #[must_use]
    pub fn ignored(mut self, ignored: bool) -> Self {
        self.is_ignored = ignored;
        self
    }

    /// Builds the Rule from the `RuleBuilder`, generating a unique ID based on the name and target.
    #[must_use]
    pub fn build(self) -> Rule {
        let unique_key = format!("{}-{}", self.name, self.target.as_ref());
        Rule {
            id: generate_id(&unique_key),
            name: self.name,
            target: self.target,
            direction: self.direction,
            is_active: self.is_active,
            is_ignored: self.is_ignored,
            amt: 0,
        }
    }
}
