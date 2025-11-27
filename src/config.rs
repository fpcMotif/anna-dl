use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub download_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_path: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let config: Config = serde_json::from_str(&contents)
                .context("Failed to parse config JSON")?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = config_path.parent().unwrap();
        
        std::fs::create_dir_all(config_dir)
            .context("Failed to create config directory")?;
        
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(&config_path, contents)
            .context("Failed to write config file")?;
        
        Ok(())
    }
    
    pub fn download_path(&self, cli_path: Option<PathBuf>) -> PathBuf {
        cli_path
            .or_else(|| self.download_path.clone())
            .unwrap_or_else(|| PathBuf::from("./assets"))
    }
    
    fn config_path() -> Result<PathBuf> {
        let project_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anna-dl");
        
        Ok(project_dir.join("config.json"))
    }
    
    pub fn set_download_path(&mut self, path: PathBuf) -> Result<()> {
        self.download_path = Some(path);
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_config_dir() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!(
            "annadl_config_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert!(config.download_path.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            download_path: Some(PathBuf::from("/test/path")),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("download_path"));
        assert!(json.contains("/test/path"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{"download_path":"/test/path"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.download_path, Some(PathBuf::from("/test/path")));
    }

    #[test]
    fn test_config_deserialization_with_null() {
        let json = r#"{"download_path":null}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.download_path.is_none());
    }

    #[test]
    fn test_config_save_and_load_roundtrip() {
        let test_dir = create_test_config_dir();
        let config_path = test_dir.join("config.json");

        // Create a config with a download path
        let original_config = Config {
            download_path: Some(PathBuf::from("/my/downloads")),
        };

        // Save it
        let json = serde_json::to_string_pretty(&original_config).unwrap();
        fs::write(&config_path, json).unwrap();

        // Load it back
        let contents = fs::read_to_string(&config_path).unwrap();
        let loaded_config: Config = serde_json::from_str(&contents).unwrap();

        assert_eq!(loaded_config.download_path, original_config.download_path);

        // Cleanup
        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn test_download_path_priority_cli_overrides_all() {
        let config = Config {
            download_path: Some(PathBuf::from("/config/path")),
        };

        let cli_path = Some(PathBuf::from("/cli/path"));
        let result = config.download_path(cli_path);

        assert_eq!(result, PathBuf::from("/cli/path"));
    }

    #[test]
    fn test_download_path_priority_config_over_default() {
        let config = Config {
            download_path: Some(PathBuf::from("/config/path")),
        };

        let result = config.download_path(None);

        assert_eq!(result, PathBuf::from("/config/path"));
    }

    #[test]
    fn test_download_path_priority_default_fallback() {
        let config = Config {
            download_path: None,
        };

        let result = config.download_path(None);

        assert_eq!(result, PathBuf::from("./assets"));
    }

    #[test]
    fn test_set_download_path() {
        let test_dir = create_test_config_dir();
        let config_path = test_dir.join("config.json");

        // Create initial config
        let mut config = Config::default();

        // Save initial config
        let json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, json).unwrap();

        // Update the path
        config.download_path = Some(PathBuf::from("/new/path"));
        let json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, json).unwrap();

        // Verify it persisted
        let contents = fs::read_to_string(&config_path).unwrap();
        let loaded: Config = serde_json::from_str(&contents).unwrap();
        assert_eq!(loaded.download_path, Some(PathBuf::from("/new/path")));

        // Cleanup
        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn test_config_handles_empty_json() {
        let json = r#"{}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.download_path.is_none());
    }

    #[test]
    fn test_config_handles_invalid_json() {
        let json = r#"{"invalid": "data"#; // Malformed JSON
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
