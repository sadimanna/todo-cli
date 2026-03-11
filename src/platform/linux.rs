use crate::config::Config;
use std::error::Error;
use std::process::Command;

pub fn notify(title: &str, message: &str) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("notify-send");
    command.arg(title).arg(message);
    run_status(command)
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
    let mut command = Command::new("paplay");
    command.arg(sound_path);
    run_status(command)
}

fn run_status(mut command: Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status: {}", status).into())
    }
}
