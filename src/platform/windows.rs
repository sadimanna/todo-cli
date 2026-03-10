use std::error::Error;
use std::process::Command;

pub fn notify(title: &str, message: &str) -> Result<(), Box<dyn Error>> {
    let command = format!(
        "New-BurntToastNotification -Text '{}','{}'",
        escape_powershell(title),
        escape_powershell(message)
    );
    run_status(
        Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(command),
    )
}

pub fn play_sound() -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn escape_powershell(input: &str) -> String {
    input.replace('\'', "''")
}

fn run_status(mut command: Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status: {}", status).into())
    }
}
