use crate::config::Config;
use std::error::Error;
use std::process::Command;

pub fn notify(title: &str, message: &str) -> Result<(), Box<dyn Error>> {
    run_status(Command::new("notify-send").arg(title).arg(message))
}

pub fn play_sound() -> Result<(), Box<dyn Error>> {
    let config = Config::load();
    if !config.enable_sound {
        return Ok(());
    }
    let sound_path = config.sound_file.trim();
    if sound_path.is_empty() {
        return Ok(());
    }
    run_status(Command::new("paplay").arg(sound_path))
}

fn run_status(mut command: Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status: {}", status).into())
    }
}
