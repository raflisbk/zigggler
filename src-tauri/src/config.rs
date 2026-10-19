use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub theme: String,
    pub locale: String,
    pub interval_secs: u64,
    pub schedule_start: String,
    pub schedule_end: String,
    pub idle_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "game-boy".into(),
            locale: "id".into(),
            interval_secs: 60,
            schedule_start: "09:00".into(),
            schedule_end: "17:00".into(),
            idle_only: false,
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("no config dir for this platform")
        .join("zzzzziglerrr")
        .join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self).expect("Config always serializes");
        fs::write(path, raw)
    }
}
