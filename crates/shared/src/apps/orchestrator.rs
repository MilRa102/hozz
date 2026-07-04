use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use config::CONF;
use db::Database;
use prefs::SettingsRegistry;
use tokio::sync::OnceCell;

use crate::{
    apps::{
        app_store::AppStore,
        prefs::{
            AllowLanCapability, AutostartCapability, ContainerCapability,
            FakeIpCapability, FindProcessCapability, GatewayCapability, PolicyCapability,
            PrefsStore, ResourceCapability, SplitRouteCapability, SystemProxyCapability,
            VaultCapability,
        },
        proxy::{ProfileStore, RuleStore},
        state::StateManager,
        tasks::BackgroundTasks,
        vault::VaultStore,
    },
    core::{dispatch::Dispatch, manager::Manager},
};

/// A singleton instance of the Application Management Orchestrator.
///
/// This module provides a centralized entry point for managing the application's lifecycle,
/// configuration, and state. It coordinates various subsystems including database storage,
/// proxy management, container capabilities, and background tasks.
///
/// The `ORCH` static variable holds the single instance of this orchestrator, ensuring
/// that only one orchestrator exists throughout the application's lifetime.
pub static ORCH: OnceCell<Arc<Orchestrator>> = OnceCell::const_new();

/// The main orchestrator struct that coordinates all application subsystems.
///
/// This struct encapsulates the state and logic required to manage the proxy, containers,
/// and other core features of the application. It uses atomic booleans for thread-safe
/// activity tracking and integrates with various stores (database, preferences, vaults)
/// to maintain a consistent application state.
///
/// # Fields
/// * `active` - An atomic boolean indicating whether the application is currently active.
/// * `connected` - An atomic boolean indicating if the proxy connection is established.
/// * `dispatch` - A dispatch handler for managing core application events and logic.
/// * `apps` - The application store containing metadata about installed applications.
/// * `profiles` - The profile store managing user-defined application profiles.
/// * `rules` - The rule store defining proxy routing and policy configurations.
/// * `vaults` - The vault store for secure storage of sensitive data (passwords, tokens).
/// * `prefs` - The preferences store holding application settings and capabilities.
/// * `state` - A reactive state manager providing metrics and current connection status to the UI.
/// * `registry` - A settings registry that exposes capabilities as features flags.
pub struct Orchestrator {
    /// An atomic boolean indicating whether the application is currently active.
    pub active: AtomicBool,

    /// An atomic boolean indicating if the proxy connection is established.
    pub connected: AtomicBool,

    /// A dispatch handler for managing core application events and logic.
    pub dispatch: Dispatch,

    /// The application store containing metadata about installed applications.
    pub apps: AppStore,

    /// The profile store managing user-defined application profiles.
    pub profiles: ProfileStore,

    /// The rule store defining proxy routing and policy configurations.
    pub rules: RuleStore,

    /// The vault store for secure storage of sensitive data (passwords, tokens).
    pub vaults: VaultStore,

    /// The preferences store holding application settings and capabilities.
    pub prefs: PrefsStore,

    /// A reactive state manager providing metrics and current connection status to the UI.
    pub state: StateManager,

    /// A settings registry that exposes capabilities as feature flags.
    pub registry: SettingsRegistry<Arc<Orchestrator>>,
}

impl Orchestrator {
    /// Initializes the application entry point and creates a new orchestrator instance.
    ///
    /// This method performs all necessary initialization steps, including:
    /// - Setting up the database storage (Sled).
    /// - Initializing the dispatch system and state manager.
    /// - Downloading the core binary if it is not present locally.
    /// - Starting the core application logic.
    /// - Registering all available capabilities with the settings registry.
    /// - Launching background tasks for metrics and health checks.
    ///
    /// # Returns
    /// * `Result<Arc<Self>>` - The initialized orchestrator instance wrapped in a Result.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Database initialization fails.
    /// - The core binary download fails (though this is logged as a warning, not a fatal error).
    /// - The orchestrator has already been initialized (singleton violation).
    ///
    /// # Example
    /// ```no_run
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// ```
    pub async fn init() -> anyhow::Result<Arc<Self>> {
        let conf = &*CONF;

        // Initialize the database storage (Sled) in the workspace data directory.
        let path_db = conf.workspace.data_dir.join("storage");
        Database::init(path_db)?;

        // Initialize the dispatch system and state manager.
        let dispatch = Dispatch::init().await?;
        let state = StateManager::init();

        // Download the core binary if it is not present locally.
        // This ensures the application has the necessary components to run.
        if !Manager::bin_path().exists() {
            Manager::download(|p| {
                let progress = p * 100.0;
                tracing::debug!(%progress, "Download progress");
            })
            .await?;
            dispatch.core.ensure_capabilities().await?;
        }

        // Start the core application logic.
        if let Err(e) = dispatch.core.start().await {
            tracing::error!(error = %e, "Core start failed");
        }

        // Initialize the apps store and set the initial connection status.
        let apps = AppStore;
        let connected = AtomicBool::new(apps.fetch().is_connected);

        // Create a new settings registry and register all available capabilities.
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
        registry.register(SplitRouteCapability);

        // Create the orchestrator instance with all initialized components.
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

        // Set the singleton instance in the static ORCH variable.
        ORCH.set(orch.clone())
            .map_err(|_| anyhow::anyhow!("Orchestrator already initialized"))?;

        // Waiting for components to start
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        // Launch background tasks for metrics collection and health checks.
        Self::launch_background();

        Ok(orch)
    }

    /// Checks if the current user has administrator privileges.
    ///
    /// # Returns
    /// * `bool` - True if the application is running with administrative rights, false otherwise.
    pub fn is_admin() -> bool {
        CONF.app.is_admin
    }

    /// Checks if the application is currently active.
    ///
    /// This method returns the current state of the application's activity flag,
    /// which indicates whether the application is running and responding to user input.
    ///
    /// # Returns
    /// * `bool` - True if the application is active, false otherwise.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Sets the activity status of the application.
    ///
    /// This method updates the internal activity flag and notifies the reactive state manager
    /// to update the UI visibility accordingly.
    ///
    /// # Arguments
    /// * `visible` - The new activity status (true for active, false for inactive).
    pub fn set_active(&self, visible: bool) {
        self.active.store(visible, Ordering::Relaxed);
        self.state.update_visibility(visible);
    }

    /// Checks if the proxy connection is currently established.
    ///
    /// This method returns the current state of the connection flag, indicating whether
    /// a valid proxy connection exists and can be used for traffic routing.
    ///
    /// # Returns
    /// * `bool` - True if the proxy is connected, false otherwise.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Sets the connection status of the proxy.
    ///
    /// This method updates the internal connection flag and can be used to reflect
    /// changes in the network state, such as disconnection or reconnection events.
    ///
    /// # Arguments
    /// * `val` - The new connection status (true for connected, false for disconnected).
    pub fn set_connected(&self, val: bool) {
        self.connected.store(val, Ordering::Relaxed);
    }
}
