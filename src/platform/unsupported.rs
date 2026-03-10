use std::error::Error;

pub fn notify(_title: &str, _message: &str) -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn play_sound() -> Result<(), Box<dyn Error>> {
    Ok(())
}
