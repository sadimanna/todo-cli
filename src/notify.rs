use crate::db::Db;
use crate::platform;
use chrono::Local;

pub fn run_notify(snooze_minutes: i64) -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::new()?;
    let now = Local::now().timestamp();
    let tasks = db.due_reminders(now)?;
    let snooze_seconds = snooze_minutes.max(0) * 60;

    for task in tasks {
        let body = match task.description.as_deref() {
            Some(desc) if !desc.is_empty() => format!("{}\n{}", task.title, desc),
            _ => task.title.clone(),
        };
        platform::notify("Task Reminder", &body)?;

        if snooze_seconds > 0 {
            let next = now + snooze_seconds;
            let _ = db.snooze_task(task.id, next);
        }

        let _ = platform::play_sound();
    }

    Ok(())
}
