use crate::config::Config;
use std::error::Error;
use std::process::Command;

pub fn notify(title: &str, message: &str) -> Result<(), Box<dyn Error>> {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape_applescript(message),
        escape_applescript(title)
    );
    let mut command = Command::new("osascript");
    command.arg("-e").arg(script);
    run_status(&mut command)
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
    let mut command = Command::new("afplay");
    command.arg(sound_path);
    run_status(&mut command)
}

fn escape_applescript(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn run_status(command: &mut Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status: {}", status).into())
    }
}
