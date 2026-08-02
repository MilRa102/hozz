/// Prompt scaffolding for the AI subsystem.
///
/// The initial implementation only exposes the module boundary and a minimal
/// public API so the rest of the refactor can evolve without coupling to a
/// concrete prompt strategy.
#[derive(Debug, Clone, Default)]
pub struct PromptRegistry;

impl PromptRegistry {
    pub fn new() -> Self {
        Self
    }
}
