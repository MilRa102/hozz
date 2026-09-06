mod control;
mod engine;
mod memory;
mod model;
mod ollama;
mod provider;
mod settings;
mod store;
#[cfg(test)]
mod test_support;
mod title;

pub use control::StreamControl;
pub use engine::{
    GenerationEvent, GenerationManager, GenerationRequest, GenerationSnapshot,
    GenerationStatus, ToolCallStatus, ToolCallView,
};
pub use memory::{
    MemoryPolicyKind, SledConversationMemory, build_memory, build_memory_with_hook,
};
pub use model::{
    Conversation, ConversationUsage, Folder, Message, MessageStatus, ProviderKind, Role,
};
pub use ollama::{OllamaModel, OllamaTagsResponse, list_ollama_models};
pub use provider::{ChatEvent, ProviderConfig};
pub use settings::AiPrefsReader;
pub use store::{ConversationStore, ConversationUsageStore, FolderStore, MessageStore};
pub use title::{generate_title, normalize_title};
