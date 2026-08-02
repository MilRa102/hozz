/// Resource scaffolding for the AI subsystem.
///
/// The initial implementation only exposes the module boundary and a minimal
/// public API so future integrations can add richer context providers without
/// increasing the refactor surface.
#[derive(Debug, Clone, Default)]
pub struct ResourceRegistry;

impl ResourceRegistry {
    pub fn new() -> Self {
        Self
    }
}
