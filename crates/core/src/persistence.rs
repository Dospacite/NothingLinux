use directories::ProjectDirs;
use nothing_protocol::AdvancedEqProfile;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use thiserror::Error;

const APP_ID: &str = "io.github.nothinglinux.nothinglinux";

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("XDG application directories are unavailable")]
    NoXdgDirectories,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("at most 20 EQ profiles are supported")]
    TooManyProfiles,
    #[error("invalid EQ profile: {0}")]
    InvalidProfile(String),
}

#[derive(Debug, Clone)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self, PersistenceError> {
        let dirs = ProjectDirs::from("io.github", "nothinglinux", "NothingLinux")
            .ok_or(PersistenceError::NoXdgDirectories)?;
        let state_dir = dirs
            .state_dir()
            .unwrap_or_else(|| dirs.data_local_dir())
            .to_path_buf();
        Ok(Self {
            config_dir: dirs.config_dir().to_path_buf(),
            state_dir,
            data_dir: dirs.data_dir().to_path_buf(),
        })
    }

    pub fn ensure(&self) -> Result<(), PersistenceError> {
        fs::create_dir_all(&self.config_dir)?;
        fs::create_dir_all(&self.state_dir)?;
        fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }
    #[must_use]
    pub fn profiles_file(&self) -> PathBuf {
        self.config_dir.join("eq-profiles.json")
    }
    #[must_use]
    pub fn diagnostics_file(&self) -> PathBuf {
        self.state_dir.join("diagnostics.log")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub start_at_login: bool,
    pub first_close_explained: bool,
    pub raw_protocol_logging: bool,
}

impl AppConfig {
    pub fn load(paths: &Paths) -> Result<Self, PersistenceError> {
        let path = paths.config_file();
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
    pub fn save(&self, paths: &Paths) -> Result<(), PersistenceError> {
        paths.ensure()?;
        atomic_json(&paths.config_file(), self)
    }

    pub fn set_autostart(&mut self, enabled: bool) -> Result<(), PersistenceError> {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or(PersistenceError::NoXdgDirectories)?;
        let directory = config_home.join("autostart");
        let file = directory.join(format!("{APP_ID}.desktop"));
        if enabled {
            fs::create_dir_all(directory)?;
            let body = format!(
                "[Desktop Entry]\nType=Application\nName=Nothing Linux\nExec=nothing-linux --background\nIcon={APP_ID}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
            );
            atomic_bytes(&file, body.as_bytes())?;
        } else if file.exists() {
            fs::remove_file(file)?;
        }
        self.start_at_login = enabled;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(transparent)]
pub struct EqProfileStore(pub Vec<AdvancedEqProfile>);

impl EqProfileStore {
    pub fn load(paths: &Paths) -> Result<Self, PersistenceError> {
        let path = paths.profiles_file();
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::import(&fs::read_to_string(path)?)
    }
    pub fn save(&self, paths: &Paths) -> Result<(), PersistenceError> {
        self.validate()?;
        paths.ensure()?;
        atomic_json(&paths.profiles_file(), self)
    }
    pub fn import(json: &str) -> Result<Self, PersistenceError> {
        let store: Self = serde_json::from_str(json)?;
        store.validate()?;
        Ok(store)
    }
    pub fn export(&self) -> Result<String, PersistenceError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn push(&mut self, profile: AdvancedEqProfile) -> Result<(), PersistenceError> {
        if self.0.len() >= 20 {
            return Err(PersistenceError::TooManyProfiles);
        }
        profile
            .validate()
            .map_err(|e| PersistenceError::InvalidProfile(e.to_string()))?;
        self.0.push(profile);
        Ok(())
    }
    fn validate(&self) -> Result<(), PersistenceError> {
        if self.0.len() > 20 {
            return Err(PersistenceError::TooManyProfiles);
        }
        for profile in &self.0 {
            profile
                .validate()
                .map_err(|e| PersistenceError::InvalidProfile(e.to_string()))?;
        }
        Ok(())
    }
}

fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), PersistenceError> {
    atomic_bytes(path, &serde_json::to_vec_pretty(value)?)
}
fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    io::Write::write_all(&mut temporary, bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[must_use]
pub fn redact_sensitive(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| {
            let trimmed = token
                .trim_matches(|character: char| !character.is_ascii_hexdigit() && character != ':');
            let is_address = trimmed.len() == 17
                && trimmed.chars().enumerate().all(|(index, value)| {
                    if index % 3 == 2 {
                        value == ':'
                    } else {
                        value.is_ascii_hexdigit()
                    }
                });
            if is_address {
                token.replace(trimmed, "XX:XX:XX:XX:XX:XX")
            } else if trimmed.len() >= 14
                && trimmed
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
            {
                token.replace(trimmed, "[redacted-identifier]")
            } else {
                token.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_defaults_are_private() {
        assert_eq!(
            AppConfig::default(),
            AppConfig {
                start_at_login: false,
                first_close_explained: false,
                raw_protocol_logging: false
            }
        );
    }
    #[test]
    fn redacts_addresses_and_serials() {
        let text = redact_sensitive("device AA:BB:CC:DD:EE:FF serial SH12345678901234");
        assert!(!text.contains("AA:BB"));
        assert!(!text.contains("SH123"));
    }
    #[test]
    fn atomic_round_trip() {
        let root = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let paths = Paths {
            config_dir: root.path().join("config"),
            state_dir: root.path().join("state"),
            data_dir: root.path().join("data"),
        };
        let config = AppConfig {
            start_at_login: true,
            ..AppConfig::default()
        };
        config.save(&paths).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            AppConfig::load(&paths).unwrap_or_else(|e| panic!("{e}")),
            config
        );
    }
}
