use db::SledManager;

#[derive(
    Debug, Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, PartialEq,
)]
pub struct App {
    pub is_connected: bool,
}

pub struct AppStore;

impl SledManager<App> for AppStore {
    const TREE_NAME: &'static str = "app";
}

impl AppStore {
    const GLOBAL_KEY: &'static str = "global";

    pub fn fetch(&self) -> App {
        self.get(Self::GLOBAL_KEY)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn update(&self, app: &App) -> anyhow::Result<()> {
        self.save(Self::GLOBAL_KEY, app)
    }
}
