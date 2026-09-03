mod control;
mod engine;
mod model;
mod provider;
mod settings;
mod store;
#[cfg(test)]
mod test_support;
mod title;

pub use control::StreamControl;
pub use engine::{
    GenerationEvent, GenerationManager, GenerationRequest, GenerationSnapshot,
};
pub use model::{Conversation, Folder, Message, MessageStatus, ProviderKind, Role};
pub use provider::{ChatEvent, ProviderConfig};
pub use settings::AiPrefsReader;
pub use store::{ConversationStore, FolderStore, MessageStore};
pub use title::{generate_title, normalize_title};
