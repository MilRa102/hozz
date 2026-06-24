pub struct SysProxyController;

impl SysProxyController {
    fn enable_proxy(&self) -> anyhow::Result<()> {
        let proxy = sysproxy::Sysproxy {
            enable: true,
            host: "127.0.0.1".to_string(),
            port: config::CONF.mihomo.mixed_port,
            bypass: "127.0.0.1,localhost".to_string(),
        };

        proxy.set_system_proxy().map_err(|e| {
            anyhow::anyhow!("Не удалось установить системный прокси: {e:?}")
        })?;

        Ok(())
    }

    fn disable_proxy(&self) -> anyhow::Result<()> {
        let proxy = sysproxy::Sysproxy {
            enable: false,
            ..Default::default()
        };

        proxy.set_system_proxy()?;
        Ok(())
    }

    pub fn toggle(&self, v: bool) -> anyhow::Result<()> {
        match v {
            true => self.enable_proxy()?,
            false => self.disable_proxy()?,
        };

        Ok(())
    }
}
