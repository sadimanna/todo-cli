use std::process::Command;

pub fn play_sound(sound_path: &str) {
    if sound_path.trim().is_empty() {
        return;
    }
    let _ = Command::new("paplay").arg(sound_path).spawn();
}
