pub trait PreferenceKey {
    const ID: &'static str;
}

#[async_trait::async_trait]
pub trait PreferenceHook<A>
where
    A: Send + Sync + 'static,
{
    fn meta(&self) -> crate::SettingMeta;

    async fn actual_state(&self, _orch: A) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    async fn before_execute(&self, _orch: A, _new: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn execute(&self, _orch: A, _new: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn after_execute(&self, _orch: A, _new: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
