use crate::task::Task;
use rusqlite::{params, Connection};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new() -> rusqlite::Result<Self> {
        let path = db_path();
        Self::new_with_path(&path)
    }

    pub fn new_with_path(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> rusqlite::Result<()> {
        let sql = include_str!("../migrations/init.sql");
        self.conn.execute_batch(sql)?;
        self.ensure_priority_column()
    }

    fn ensure_priority_column(&self) -> rusqlite::Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(tasks)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_priority = false;
        for name in rows {
            if name? == "priority" {
                has_priority = true;
                break;
            }
        }
        if !has_priority {
            self.conn.execute(
                "ALTER TABLE tasks ADD COLUMN priority INTEGER DEFAULT 1",
                [],
            )?;
        }
        Ok(())
    }

    pub fn add_task(
        &self,
        title: &str,
        description: Option<&str>,
        priority: i64,
        deadline: Option<i64>,
        reminder: Option<i64>,
    ) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO tasks (title, description, priority, deadline, reminder, completed) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![title, description, priority, deadline, reminder],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_tasks(&self) -> rusqlite::Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, priority, deadline, reminder, completed FROM tasks ORDER BY completed ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                priority: row.get::<_, Option<i64>>(3)?.unwrap_or(1),
                deadline: row.get(4)?,
                reminder: row.get(5)?,
                completed: row.get::<_, i64>(6)? != 0,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    pub fn mark_done(&self, id: i64) -> rusqlite::Result<usize> {
        self.conn
            .execute("UPDATE tasks SET completed = 1 WHERE id = ?1", params![id])
    }

    pub fn delete_task(&self, id: i64) -> rusqlite::Result<usize> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
    }

    pub fn update_task(
        &self,
        id: i64,
        title: &str,
        priority: i64,
        deadline: Option<i64>,
        reminder: Option<i64>,
    ) -> rusqlite::Result<usize> {
        self.conn.execute(
            "UPDATE tasks SET title = ?1, priority = ?2, deadline = ?3, reminder = ?4 WHERE id = ?5",
            params![title, priority, deadline, reminder, id],
        )
    }

    pub fn snooze_task(&self, id: i64, next_reminder: i64) -> rusqlite::Result<usize> {
        self.conn.execute(
            "UPDATE tasks SET reminder = ?1 WHERE id = ?2",
            params![next_reminder, id],
        )
    }

    pub fn due_reminders(&self, now: i64) -> rusqlite::Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, priority, deadline, reminder, completed FROM tasks \
             WHERE reminder IS NOT NULL AND reminder <= ?1 AND completed = 0 \
             ORDER BY reminder ASC",
        )?;
        let rows = stmt.query_map(params![now], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                priority: row.get::<_, Option<i64>>(3)?.unwrap_or(1),
                deadline: row.get(4)?,
                reminder: row.get(5)?,
                completed: row.get::<_, i64>(6)? != 0,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }
}

pub fn db_path() -> PathBuf {
    if let Ok(path) = env::var("TODO_DB_PATH") {
        return PathBuf::from(path);
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".todo").join("tasks.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn crud_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let db = Db::new_with_path(&path).unwrap();

        let id = db.add_task("Test", None, 1, Some(123), Some(120)).unwrap();
        let tasks = db.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id);

        db.mark_done(id).unwrap();
        let tasks = db.list_tasks().unwrap();
        assert!(tasks[0].completed);

        db.delete_task(id).unwrap();
        let tasks = db.list_tasks().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn due_reminders_filters() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let db = Db::new_with_path(&path).unwrap();

        db.add_task("Soon", None, 1, None, Some(100)).unwrap();
        db.add_task("Later", None, 1, None, Some(200)).unwrap();

        let due = db.due_reminders(150).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].title, "Soon");
    }
}
