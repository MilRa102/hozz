use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use config::CONF;
use prefs::SettingsRegistry;
use tokio::sync::OnceCell;

use crate::{
    app::state::StateManager,
    apps::prefs::{
        AllowLanCapability, AutostartCapability, ContainerCapability, FakeIpCapability,
        FindProcessCapability, GatewayCapability, PolicyCapability, ResourceCapability,
        SystemProxyCapability, VaultCapability,
    },
    core::{dispatch::Dispatch, manager::Manager},
    db::{
        Database, app::AppStore, prefs::PrefsStore, profile::ProfileStore,
        rule::RuleStore, vault::VaultStore,
    },
    infra::tasks::BackgroundTasks,
};

pub static ORCH: OnceCell<Arc<Orchestrator>> = OnceCell::const_new();

pub struct Orchestrator {
    // Быстрые метки по состоянию окна и подключению прокси
    pub active: AtomicBool,
    connected: AtomicBool,
    // Управление ядром и конфигом
    pub dispatch: Dispatch,
    // Постоянное хранилище (Sled)
    pub apps: AppStore,
    pub profiles: ProfileStore,
    pub rules: RuleStore,
    pub vaults: VaultStore,
    pub prefs: PrefsStore,
    // Реактивное состояние для UI (метрики, текущие прокси)
    pub state: StateManager,

    pub registry: SettingsRegistry<Arc<Orchestrator>>,
}

impl Orchestrator {
    /// Initializing the application entry point
    ///
    /// # Returns
    /// Application Management Orchestrator
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// Orcestrator::init().await.unwrap();
    /// ```
    pub async fn init() -> anyhow::Result<Arc<Self>> {
        let conf = &*CONF;

        let path_db = conf.workspace.data_dir.join("storage");
        Database::init(path_db)?;

        let dispatch = Dispatch::init().await?;
        let state = StateManager::init();

        if !Manager::bin_path().exists() {
            Manager::download(|p| {
                let progress = p * 100.0;
                tracing::debug!(%progress, "Download progress");
            })
            .await?;
        }

        if let Err(e) = dispatch.core.start().await {
            tracing::error!(error = %e, "Core start failed");
        }

        let apps = AppStore;
        let connected = AtomicBool::new(apps.fetch().is_connected);

        let mut registry = SettingsRegistry::<Arc<Orchestrator>>::new();
        registry.register(ContainerCapability);
        registry.register(GatewayCapability);
        registry.register(PolicyCapability);
        registry.register(ResourceCapability);
        registry.register(VaultCapability);
        registry.register(AutostartCapability);
        registry.register(AllowLanCapability);
        registry.register(FakeIpCapability);
        registry.register(FindProcessCapability);
        registry.register(SystemProxyCapability);

        let orch = Arc::new(Self {
            active: AtomicBool::new(true),
            connected,
            dispatch,
            apps,
            profiles: ProfileStore,
            rules: RuleStore,
            vaults: VaultStore,
            prefs: PrefsStore,
            state,
            registry,
        });

        ORCH.set(orch.clone())
            .map_err(|_| anyhow::anyhow!("Orchestrator already initialized"))?;

        // Запускаем фоновые задачи (метрики, проверки здоровья)
        Self::launch_background();

        Ok(orch)
    }

    pub fn is_admin() -> bool {
        CONF.app.is_admin
    }

    /// Application activity flag
    ///
    /// # Returns
    /// The application is currently active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Set application activity
    ///
    /// # Arguments
    /// * `val` - The new activity status.
    pub fn set_active(&self, visible: bool) {
        self.active.store(visible, Ordering::Relaxed);
        self.state.update_visibility(visible);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn set_connected(&self, val: bool) {
        self.connected.store(val, Ordering::Relaxed);
    }
}
