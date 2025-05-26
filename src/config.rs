use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub model: String,
    pub uri: String,
    pub system: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            model: "deepseek-r1:1.5b".into(),
            uri: "http://localhost:11434".into(),
            system: None,
        }
    }
}

fn config_path() -> io::Result<PathBuf> {
    let base = BaseDirs::new()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find home directory"))?;
    let dir = base.config_dir().join("oma");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("config.toml"))
}

pub fn load_config() -> io::Result<Config> {
    let path = config_path()?;
    if path.exists() {
        let s = fs::read_to_string(&path)?;
        let cfg = toml::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(cfg)
    } else {
        let cfg = Config::default();
        let toml = toml::to_string_pretty(&cfg)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, toml)?;
        Ok(cfg)
    }
}
