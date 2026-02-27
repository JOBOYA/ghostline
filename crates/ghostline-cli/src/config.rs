use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub auth: AuthConfig,
    pub proxy: ProxyConfig,
    pub viewer: ViewerConfig,
    pub recording: RecordingConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub claude_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub port: u16,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerConfig {
    pub port: u16,
    pub auto_open_browser: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub output_dir: String,
    pub scrub: bool,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub colors: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth: AuthConfig { claude_token: None },
            proxy: ProxyConfig {
                port: 9000,
                target: "https://api.anthropic.com".to_string(),
            },
            viewer: ViewerConfig {
                port: 5173,
                auto_open_browser: true,
            },
            recording: RecordingConfig {
                output_dir: "~/.ghostline/runs".to_string(),
                scrub: true,
                default_model: "claude-3-haiku-20240307".to_string(),
            },
            display: DisplayConfig { colors: true },
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home dir")
            .join(".ghostline")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn runs_dir() -> PathBuf {
        dirs::home_dir()
            .expect("no home dir")
            .join(".ghostline")
            .join("runs")
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if path.exists() {
            Self::load(&path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        // Set restrictive permissions on config (contains token)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        self.auth.claude_token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = Config::default();
        assert_eq!(cfg.proxy.port, 9000);
        assert_eq!(cfg.viewer.port, 5173);
        assert!(cfg.recording.scrub);
    }

    #[test]
    fn test_config_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.auth.claude_token = Some("test-token".to_string());
        cfg.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.proxy.port, cfg.proxy.port);
        assert_eq!(loaded.auth.claude_token, Some("test-token".to_string()));
    }
}
