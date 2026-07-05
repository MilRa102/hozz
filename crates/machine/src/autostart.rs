use auto_launch::{AutoLaunch, AutoLaunchBuilder};

pub struct AutostartApp;

impl AutostartApp {
    fn get_manager(&self) -> anyhow::Result<AutoLaunch> {
        let path = std::env::current_exe()?;
        let launch = AutoLaunchBuilder::new()
            .set_app_name("hozz")
            .set_app_path(
                path.to_str()
                    .ok_or(anyhow::anyhow!("Failed to get path"))?,
            )
            .set_args(&["--minimized"])
            .build()?;

        Ok(launch)
    }

    pub fn is_enable(&self) -> anyhow::Result<bool> {
        let mgr = self.get_manager()?;
        Ok(mgr.is_enabled()?)
    }

    pub fn toggle(&self, v: bool) -> anyhow::Result<()> {
        let mgr = self.get_manager()?;

        match v {
            true => {
                if !mgr.is_enabled().unwrap_or(true) {
                    mgr.enable()?
                }
            },
            false => {
                if mgr.is_enabled().unwrap_or(false) {
                    mgr.disable()?
                }
            },
        }

        tracing::info!(state = %v, "Auto-Start toggled");
        Ok(())
    }
}
