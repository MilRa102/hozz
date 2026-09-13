use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    MemoryMapEntry, MemoryMapStore,
    embedding::{compute_blake3_hash, embed_memory_map_document_sync},
};

pub struct MemoryWatcherHandle {
    _watcher: RecommendedWatcher,
}

pub fn sync_embed_dir(embed_dir: &Path) -> anyhow::Result<()> {
    if !embed_dir.exists() {
        fs::create_dir_all(embed_dir)?;
        return Ok(());
    }

    let store = MemoryMapStore;
    let mut found_file_ids = HashSet::new();

    let read_dir = fs::read_dir(embed_dir)?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(file_name) = path.file_name().and_then(|s| s.to_str())
        {
            let file_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(file_name)
                .to_string();

            found_file_ids.insert(file_id.clone());

            if let Ok(content) = fs::read_to_string(&path) {
                if content.trim().is_empty() {
                    let _ = store.remove_for_conversation(&file_id);
                    continue;
                }

                let hash = compute_blake3_hash(&content);
                let existing = store.get_by_conversation(&file_id).ok().flatten();

                if existing.as_ref().map(|e| e.hash.as_str()) != Some(&hash)
                    && let Ok(embed) = embed_memory_map_document_sync(&content)
                {
                    let map_entry = MemoryMapEntry::new(&file_id, content, hash, embed);
                    let _ = store.replace_for_conversation(&file_id, &map_entry);
                }
            }
        }
    }

    if let Ok(all_entries) = store.all() {
        for entry in all_entries {
            if !found_file_ids.contains(&entry.conversation_id) {
                let _ = store.remove_for_conversation(&entry.conversation_id);
            }
        }
    }

    Ok(())
}

pub fn start_memory_watcher(embed_dir: PathBuf) -> anyhow::Result<MemoryWatcherHandle> {
    fs::create_dir_all(&embed_dir)?;

    if let Err(err) = sync_embed_dir(&embed_dir) {
        warn!(error = %err, "Initial memory embed_dir sync failed");
    }

    let (tx, mut rx) = mpsc::channel::<()>(100);
    let watcher_tx = tx.clone();

    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res
                && matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                )
            {
                let _ = watcher_tx.try_send(());
            }
        })?;

    watcher.watch(&embed_dir, RecursiveMode::Recursive)?;

    let dir_clone = embed_dir.clone();
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            while rx.try_recv().is_ok() {}

            let dir = dir_clone.clone();
            let res = tokio::task::spawn_blocking(move || sync_embed_dir(&dir)).await;
            if let Err(err) = res {
                warn!(error = %err, "Failed to sync memory embed_dir on change");
            }
        }
    });

    info!(path = ?embed_dir, "Memory folder watcher started");

    Ok(MemoryWatcherHandle { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::init_db;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn syncs_file_in_embed_dir_to_sled() {
        init_db();
        let temp_dir =
            std::env::temp_dir().join(format!("test_embed_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("test_conv.md");
        fs::write(&file_path, "test memory content").unwrap();

        sync_embed_dir(&temp_dir).unwrap();

        let store = MemoryMapStore;
        let entry = store
            .get_by_conversation("test_conv")
            .unwrap()
            .unwrap();
        assert_eq!(entry.content, "test memory content");
        assert!(!entry.hash.is_empty());

        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
