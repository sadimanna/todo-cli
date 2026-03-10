use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
const DEFAULT_SOUND: &str = "/usr/share/sounds/freedesktop/stereo/message.oga";
#[cfg(target_os = "macos")]
const DEFAULT_SOUND: &str = "/System/Library/Sounds/Ping.aiff";
#[cfg(target_os = "windows")]
const DEFAULT_SOUND: &str = "";
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const DEFAULT_SOUND: &str = "";

#[cfg(target_os = "windows")]
const DEFAULT_ENABLE_SOUND: bool = false;
#[cfg(not(target_os = "windows"))]
const DEFAULT_ENABLE_SOUND: bool = true;

#[derive(Debug, Clone)]
pub struct Config {
    pub enable_sound: bool,
    pub sound_file: String,
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match fs::read_to_string(path) {
            Ok(contents) => parse_config(&contents),
            Err(_) => Self::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_sound: DEFAULT_ENABLE_SOUND,
            sound_file: DEFAULT_SOUND.to_string(),
        }
    }
}

fn config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".todo").join("config.toml")
}

fn parse_config(input: &str) -> Config {
    let mut config = Config::default();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let key = parts.next().map(str::trim);
        let value = parts.next().map(str::trim);
        let (key, value) = match (key, value) {
            (Some(k), Some(v)) => (k, v),
            _ => continue,
        };
        match key {
            "enable_sound" => {
                if let Some(parsed) = parse_bool(value) {
                    config.enable_sound = parsed;
                }
            }
            "sound_file" => {
                if let Some(parsed) = parse_string(value) {
                    config.sound_file = parsed;
                }
            }
            _ => {}
        }
    }
    config
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        Some(trimmed[1..trimmed.len() - 1].to_string())
    } else if trimmed.starts_with('"') {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_defaults() {
        let config = parse_config("");
        assert_eq!(config.enable_sound, DEFAULT_ENABLE_SOUND);
        assert_eq!(config.sound_file, DEFAULT_SOUND);
    }

    #[test]
    fn parse_config_values() {
        let config = parse_config(
            "enable_sound = false\n\
            sound_file = \"/tmp/test.oga\"\n",
        );
        assert!(!config.enable_sound);
        assert_eq!(config.sound_file, "/tmp/test.oga");
    }
}
