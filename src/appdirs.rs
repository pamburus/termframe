// std imports
use std::path::PathBuf;

/// A structure to hold application directories.
#[allow(dead_code)]
pub struct AppDirs {
    /// Directory for cache files.
    pub cache_dir: PathBuf,
    /// Directory for configuration files.
    pub config_dir: PathBuf,
    /// Directories for system-wide configuration files.
    pub system_config_dirs: Vec<PathBuf>,
}

impl AppDirs {
    /// Creates a new `AppDirs` instance with the given application name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the application.
    ///
    /// # Returns
    ///
    /// An `Option` containing the `AppDirs` instance if successful, or `None` if any directory could not be determined.
    pub fn new(name: &str) -> Option<Self> {
        let cache_dir = sys::cache_dir()?.join(name);
        let config_dir = sys::config_dir()?.join(name);
        let system_config_dirs = sys::system_config_dirs()
            .into_iter()
            .map(|d| d.join(name))
            .collect();
        Some(Self {
            cache_dir,
            config_dir,
            system_config_dirs,
        })
    }
}

#[cfg(target_os = "macos")]
mod sys {
    use super::*;
    use std::env;

    /// Returns the cache directory path.
    ///
    /// # Returns
    ///
    /// An `Option` containing the path to the cache directory, or `None` if it could not be determined.
    pub(crate) fn cache_dir() -> Option<PathBuf> {
        env::var_os("XDG_CACHE_HOME")
            .and_then(dirs_sys::is_absolute_path)
            .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
    }

    /// Returns the configuration directory path.
    ///
    /// # Returns
    ///
    /// An `Option` containing the path to the configuration directory, or `None` if it could not be determined.
    pub(crate) fn config_dir() -> Option<PathBuf> {
        env::var_os("XDG_CONFIG_HOME")
            .and_then(dirs_sys::is_absolute_path)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
    }

    /// Returns the system-wide configuration directories.
    ///
    /// # Returns
    ///
    /// A vector containing the paths to the system-wide configuration directories.
    pub(crate) fn system_config_dirs() -> Vec<PathBuf> {
        vec![PathBuf::from("/etc")]
    }
}

#[cfg(not(target_os = "macos"))]
mod sys {
    use super::*;

    /// Returns the configuration directory path.
    ///
    /// # Returns
    ///
    /// An `Option` containing the path to the configuration directory, or `None` if it could not be determined.
    pub(crate) fn config_dir() -> Option<PathBuf> {
        dirs::config_dir()
    }

    /// Returns the cache directory path.
    ///
    /// # Returns
    ///
    /// An `Option` containing the path to the cache directory, or `None` if it could not be determined.
    pub(crate) fn cache_dir() -> Option<PathBuf> {
        dirs::cache_dir()
    }

    /// Returns the system-wide configuration directories for non-Windows systems.
    ///
    /// # Returns
    ///
    /// A vector containing the paths to the system-wide configuration directories.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn system_config_dirs() -> Vec<PathBuf> {
        vec![PathBuf::from("/etc")]
    }

    /// Returns the system-wide configuration directories for Windows systems.
    ///
    /// # Returns
    ///
    /// A vector containing the paths to the system-wide configuration directories.
    #[cfg(target_os = "windows")]
    pub(crate) fn system_config_dirs() -> Vec<PathBuf> {
        use known_folders::{KnownFolder, get_known_folder_path};

        get_known_folder_path(KnownFolder::ProgramData)
            .into_iter()
            .collect()
    }
}
