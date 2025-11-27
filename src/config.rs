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
