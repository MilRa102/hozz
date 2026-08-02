use crate::{ai::AiRegistry, apps::Orchestrator};

#[test]
fn ai_registry_trait_is_available() {
    fn assert_impl<T: AiRegistry>() {}

    assert_impl::<Orchestrator>();
}
