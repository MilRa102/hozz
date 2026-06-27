mod pool;
mod sled;

// Re-export
/// The main module for database operations using Sled.
///
/// This module provides utilities for interacting with a Sled database, including
/// serialization/deserialization helpers and a trait for managing data storage.
pub use pool::Database;
pub use sled::{SledManager, decode};
