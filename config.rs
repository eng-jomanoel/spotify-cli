use anyhow::{bail, Result};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default = "default_port")]
    pub redirect_port: u16,
}

fn default_port() -> u16 {
    8888
}

impl Config {
    pub fn config_dir() -> PathBuf {
        config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("spotify-cli")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn token_path() -> PathBuf {
        Self::config_dir().join("tokens.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            // Create default config and prompt user
            fs::create_dir_all(Self::config_dir())?;
            let example = r#"# Spotify CLI Configuration
# Get your credentials at https://developer.spotify.com/dashboard

client_id = "YOUR_CLIENT_ID_HERE"
client_secret = "YOUR_CLIENT_SECRET_HERE"
redirect_port = 8888
"#;
            fs::write(&path, example)?;
            bail!(
                "Config file created at {:?}\nPlease fill in your Spotify credentials.\nGet them at https://developer.spotify.com/dashboard",
                path
            );
        }

        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;

        if config.client_id == "YOUR_CLIENT_ID_HERE" {
            bail!(
                "Please set your Spotify credentials in {:?}",
                path
            );
        }

        Ok(config)
    }
}
