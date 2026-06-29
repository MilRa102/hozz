use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use config::CONF;
use futures::StreamExt;
use reqwest::header::USER_AGENT;
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::{Child, Command},
    sync::Mutex,
    time::sleep,
};

use super::models::conf::Mihomo;

#[derive(Debug, Default, Clone)]
pub struct Manager {
    process: Arc<Mutex<Option<Child>>>,
}

impl Manager {
    pub fn new() -> Self {
        Self::default()
    }

    fn gh_target() -> (&'static str, &'static str, &'static str) {
        let gh_os = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "windows",
            _ => "linux",
        };

        let gh_arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            arch => arch,
        };

        let extension = if std::env::consts::OS == "windows" {
            ".exe"
        } else {
            ""
        };

        (gh_os, gh_arch, extension)
    }

    fn pid_path() -> PathBuf {
        CONF.workspace.data_dir.join("mihomo.pid")
    }

    pub fn bin_path() -> PathBuf {
        let (gh_os, gh_arch, extension) = Self::gh_target();
        let version = &CONF.mihomo.version;

        let file_name = format!("mihomo-{version}-{gh_os}-{gh_arch}{extension}");
        CONF.workspace.bin_dir.join(file_name)
    }

    pub async fn download<F>(on_progress: F) -> Result<()>
    where
        F: Fn(f64) + Send + 'static,
    {
        let bin_path = Self::bin_path();
        if bin_path.exists() {
            return Ok(());
        }

        let (gh_os, gh_arch, extension) = Self::gh_target();
        let version = CONF.mihomo.version.clone();

        let url = format!(
            "https://github.com/MilRa102/hozz-core/releases/download/{version}/mihomo-{gh_os}-{gh_arch}{extension}"
        );

        let client = reqwest::Client::new();
        let res = client
            .get(url)
            .header(USER_AGENT, "Hozz-Manager")
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Failed to download mihomo from GitHub: HTTP {}",
                    res.status()
                ),
            )
            .into());
        }

        let total_size = res.content_length().unwrap_or(0);

        let mut file = fs::File::create(&bin_path).await?;
        let mut downloaded: u64 = 0;
        let mut stream = res.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                on_progress(downloaded as f64 / total_size as f64);
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o775))
                .await?;
        }

        Ok(())
    }

    /// Setting TUN usage privileges, only Linux
    #[cfg(target_os = "linux")]
    pub async fn ensure_capabilities(&self) -> Result<()> {
        let bin_path = Self::bin_path().canonicalize()?;
        tracing::info!("Setting capabilities for binary");

        let caps = "cap_net_admin,cap_net_bind_service=+ep";

        let status = Command::new("pkexec")
            .arg("setcap")
            .arg(caps)
            .arg(&bin_path)
            .status()
            .await
            .context("Failed to execute setcap command")?;

        if status.success() {
            tracing::info!("Capabilities set successfully");
            Ok(())
        } else {
            let error = "Failed to set capabilities. Make sure you have sudo rights or the app is running as root.";
            tracing::warn!(%error, "Failed to set capabilities");
            anyhow::bail!(error)
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn ensure_capabilities(&self) -> Result<()> {
        tracing::info!("Capabilities are not required on this OS. Skipping..");
        Ok(())
    }

    /// Checks for the existence of a PID file and terminates the process if it exists.
    async fn cleanup_orphan(&self) -> Result<()> {
        let path = Self::pid_path();
        if !path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&path).await?;
        if let Ok(pid) = content.trim().parse::<i32>() {
            tracing::info!(%pid, "Found PID file. Checking process..");

            #[cfg(unix)]
            {
                let status = Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .status()
                    .await
                    .context("Process is not found")?;

                if status.success() {
                    tracing::info!(%pid, "Orphan Mihomo is alive. Killing..");
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .status()
                        .await;
                    sleep(Duration::from_millis(300)).await;
                }
            }

            #[cfg(windows)]
            {
                let status = Command::new("taskkill")
                    .arg("/PID")
                    .arg(pid.to_string())
                    .arg("/T")
                    .arg("/F")
                    .status()
                    .await
                    .context("Process is not found")?;

                if status.success() {
                    tracing::warn!(%pid, "Orphan Mihomo is alive. Killing..");
                    sleep(Duration::from_millis(300)).await;
                }
            }
        }

        let _ = fs::remove_file(path).await;
        Ok(())
    }

    /// Starts the Mihomo process.
    pub async fn start(&self) -> Result<()> {
        let mut process_guard = self.process.lock().await;

        if let Some(mut child) = process_guard.take() {
            if let Err(error) = child.kill().await {
                tracing::warn!(%error, "kill process failed");
            }
            if let Err(error) = child.wait().await {
                tracing::warn!(%error, "wait process failed");
            }
        }

        if let Err(error) = self.cleanup_orphan().await {
            tracing::warn!(%error, "cleanup orphan failed");
        }

        let bin_path = Self::bin_path();
        let config_path = Mihomo::config_path();

        let mut cmd = Command::new(bin_path);
        cmd.arg("-d")
            .arg(&CONF.workspace.data_dir)
            .arg("-f")
            .arg(&config_path)
            .current_dir(&CONF.workspace.bin_dir)
            .kill_on_drop(true);

        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let child = cmd
            .spawn()
            .context("Failed to spawn Mihomo binary")?;

        if let Some(pid) = child.id()
            && let Err(error) = fs::write(Self::pid_path(), pid.to_string()).await
        {
            tracing::error!(%error, "failed to write PID to file");
        }

        *process_guard = Some(child);
        tracing::info!("Mihomo started successfully.");
        Ok(())
    }

    /// Stops the Mihomo process.
    pub async fn stop(&self) {
        let mut process_guard = self.process.lock().await;
        if let Some(mut child) = process_guard.take() {
            match child.kill().await {
                Ok(()) => {
                    let _ = child.wait().await;
                    tracing::info!("Mihomo stopped successfully.");
                },
                Err(e) => tracing::error!(error = %e, "Failed to stop Mihomo"),
            }
        }

        let _ = self.cleanup_orphan().await;
        tracing::info!("Mihomo stopped and cleaned up");
    }
}
