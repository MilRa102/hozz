use std::{fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_bin_dir")]
    pub bin_dir: PathBuf,
    #[serde(default)]
    pub log_dir: Option<PathBuf>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            bin_dir: default_bin_dir(),
            log_dir: None,
        }
    }
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "Nodes", "hozz")
        .expect("Failed to determine project directories")
}

fn default_data_dir() -> PathBuf {
    project_dirs().config_dir().to_path_buf()
}

fn default_bin_dir() -> PathBuf {
    project_dirs().data_dir().to_path_buf()
}

impl WorkspaceConfig {
    pub(crate) fn ensure_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.bin_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for dir in [&self.data_dir, &self.bin_dir] {
                fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
            }
        }

        Ok(())
    }

    pub fn get_log_dir(&self) -> io::Result<PathBuf> {
        if let Some(dir) = &self.log_dir {
            fs::create_dir_all(dir)?;
            Ok(dir.clone())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Log directory is not configured",
            ))
        }
    }
}
