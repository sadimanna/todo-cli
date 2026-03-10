use crate::db::Db;
use chrono::Local;
use notify_rust::Notification;

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
        let mut notification = Notification::new();
        notification.summary("Task Reminder").body(&body);
        if snooze_seconds > 0 {
            notification.action("snooze", &format!("Snooze {}m", snooze_minutes));
        }
        notification.show()?;

        if snooze_seconds > 0 {
            let next = now + snooze_seconds;
            let _ = db.snooze_task(task.id, next);
        }
    }

    Ok(())
}
