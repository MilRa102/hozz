use std::sync::Once;

static INIT: Once = Once::new();

/// Initializes a process-wide temporary Sled database for tests. Safe to call
/// from every test — only the first call actually opens the database.
pub(crate) fn init_db() {
    INIT.call_once(|| {
        let path = std::env::temp_dir().join(format!("hozz-ai-tests-{}", std::process::id()));
        db::Database::init(path).expect("failed to init test database");
    });
}
