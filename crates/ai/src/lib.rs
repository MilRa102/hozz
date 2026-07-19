mod control;
mod engine;
mod model;
mod provider;
mod settings;
mod store;
#[cfg(test)]
mod test_support;

pub use control::StreamControl;
pub use engine::{GenerationManager, GenerationSnapshot};
pub use model::{Conversation, Folder, Message, MessageStatus, ProviderKind, Role};
pub use provider::{ChatEvent, ProviderConfig};
pub use settings::AiPrefsReader;
pub use store::{ConversationStore, FolderStore, MessageStore};
