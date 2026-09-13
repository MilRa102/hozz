mod control;
mod embedding;
mod engine;
mod memory;
mod memory_map;
mod model;
mod ollama;
mod provider;
mod settings;
mod store;
#[cfg(test)]
mod test_support;
mod title;
mod watcher;

pub use control::StreamControl;
pub use embedding::{
    MemoryMapDocument, MemoryMapHit, memory_map_context, search_memory_map,
};
pub use engine::{
    GenerationEvent, GenerationManager, GenerationRequest, GenerationSnapshot,
    GenerationStatus, ToolCallStatus, ToolCallView,
};
pub use memory::{
    MemoryPolicyKind, SledConversationMemory, build_memory, build_memory_with_hook,
};
pub use memory_map::{MemoryMapEntry, MemoryMapStore};
pub use model::{
    Conversation, ConversationUsage, Folder, Message, MessageStatus, ProviderKind, Role,
};
pub use ollama::{OllamaModel, OllamaTagsResponse, list_ollama_models};
pub use provider::{ChatEvent, ProviderConfig};
pub use settings::AiPrefsReader;
pub use store::{ConversationStore, ConversationUsageStore, FolderStore, MessageStore};
pub use title::{generate_title, normalize_title};
pub use watcher::{MemoryWatcherHandle, start_memory_watcher, sync_embed_dir};
