use crate::project::Project;
use crate::task::{status_from_db, status_to_db, Status, Task};
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
        self.ensure_priority_column()?;
        self.ensure_projects_table()?;
        self.ensure_project_column()?;
        self.ensure_status_column()?;
        self.ensure_default_project()
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

    fn ensure_projects_table(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (\
                id INTEGER PRIMARY KEY AUTOINCREMENT,\
                name TEXT UNIQUE NOT NULL,\
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP\
            );",
        )?;
        Ok(())
    }

    fn ensure_project_column(&self) -> rusqlite::Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(tasks)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_project = false;
        for name in rows {
            if name? == "project_id" {
                has_project = true;
                break;
            }
        }
        if !has_project {
            self.conn
                .execute("ALTER TABLE tasks ADD COLUMN project_id INTEGER", [])?;
        }
        Ok(())
    }

    fn ensure_status_column(&self) -> rusqlite::Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(tasks)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_status = false;
        for name in rows {
            if name? == "status" {
                has_status = true;
                break;
            }
        }
        if !has_status {
            self.conn
                .execute("ALTER TABLE tasks ADD COLUMN status TEXT", [])?;
        }
        self.conn.execute(
            "UPDATE tasks SET status = CASE WHEN completed = 1 THEN 'DONE' ELSE 'TODO' END \
             WHERE status IS NULL OR status = ''",
            [],
        )?;
        self.conn.execute(
            "UPDATE tasks SET completed = CASE WHEN status = 'DONE' THEN 1 ELSE 0 END",
            [],
        )?;
        Ok(())
    }

    fn ensure_default_project(&self) -> rusqlite::Result<()> {
        let default_id = match self.project_id_by_name("All")? {
            Some(id) => id,
            None => {
                self.conn
                    .execute("INSERT INTO projects (name) VALUES ('All')", [])?;
                self.conn.last_insert_rowid()
            }
        };
        self.conn.execute(
            "UPDATE tasks SET project_id = ?1 WHERE project_id IS NULL",
            params![default_id],
        )?;
        Ok(())
    }

    pub fn default_project_id(&self) -> rusqlite::Result<i64> {
        match self.project_id_by_name("All")? {
            Some(id) => Ok(id),
            None => {
                self.conn
                    .execute("INSERT INTO projects (name) VALUES ('All')", [])?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    pub fn add_task(
        &self,
        title: &str,
        description: Option<&str>,
        project_id: Option<i64>,
        priority: i64,
        deadline: Option<i64>,
        reminder: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let project_id = match project_id {
            Some(id) => Some(id),
            None => Some(self.default_project_id()?),
        };
        self.conn.execute(
            "INSERT INTO tasks (title, description, project_id, status, priority, deadline, reminder, completed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                title,
                description,
                project_id,
                status_to_db(Status::Todo),
                priority,
                deadline,
                reminder
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_tasks(&self) -> rusqlite::Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, project_id, status, priority, deadline, reminder, completed \
             FROM tasks ORDER BY CASE status WHEN 'TODO' THEN 0 WHEN 'IN_PROGRESS' THEN 1 WHEN 'DONE' THEN 2 ELSE 3 END, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                project_id: row.get(3)?,
                status: status_from_db(row.get(4)?),
                priority: row.get::<_, Option<i64>>(5)?.unwrap_or(1),
                deadline: row.get(6)?,
                reminder: row.get(7)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    pub fn list_tasks_for_project(&self, project_id: i64) -> rusqlite::Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, project_id, status, priority, deadline, reminder, completed \
             FROM tasks WHERE project_id = ?1 ORDER BY CASE status WHEN 'TODO' THEN 0 WHEN 'IN_PROGRESS' THEN 1 WHEN 'DONE' THEN 2 ELSE 3 END, id ASC",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                project_id: row.get(3)?,
                status: status_from_db(row.get(4)?),
                priority: row.get::<_, Option<i64>>(5)?.unwrap_or(1),
                deadline: row.get(6)?,
                reminder: row.get(7)?,
            })
        })?;

        let mut tasks = Vec::new();
        for task in rows {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    pub fn list_projects(&self) -> rusqlite::Result<Vec<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM projects ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        let mut projects = Vec::new();
        for project in rows {
            projects.push(project?);
        }
        Ok(projects)
    }

    pub fn create_project(&self, name: &str) -> rusqlite::Result<i64> {
        self.conn
            .execute("INSERT INTO projects (name) VALUES (?1)", params![name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_project(&self, id: i64, name: &str) -> rusqlite::Result<usize> {
        self.conn.execute(
            "UPDATE projects SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
    }

    pub fn delete_project(&mut self, id: i64) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE tasks SET project_id = NULL WHERE project_id = ?1",
            params![id],
        )?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn project_id_by_name(&self, name: &str) -> rusqlite::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM projects WHERE name = ?1")?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    pub fn set_task_project(&self, id: i64, project_id: Option<i64>) -> rusqlite::Result<usize> {
        let project_id = match project_id {
            Some(id) => Some(id),
            None => Some(self.default_project_id()?),
        };
        self.conn.execute(
            "UPDATE tasks SET project_id = ?1 WHERE id = ?2",
            params![project_id, id],
        )
    }

    pub fn mark_done(&self, id: i64) -> rusqlite::Result<usize> {
        self.set_task_status(id, Status::Done)
    }

    pub fn set_task_status(&self, id: i64, status: Status) -> rusqlite::Result<usize> {
        let completed = matches!(status, Status::Done);
        self.conn.execute(
            "UPDATE tasks SET status = ?1, completed = ?2 WHERE id = ?3",
            params![status_to_db(status), if completed { 1 } else { 0 }, id],
        )
    }

    pub fn delete_task(&self, id: i64) -> rusqlite::Result<usize> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
    }

    pub fn update_task(
        &self,
        id: i64,
        title: &str,
        project_id: Option<i64>,
        priority: i64,
        deadline: Option<i64>,
        reminder: Option<i64>,
    ) -> rusqlite::Result<usize> {
        let project_id = match project_id {
            Some(id) => Some(id),
            None => Some(self.default_project_id()?),
        };
        self.conn.execute(
            "UPDATE tasks SET title = ?1, project_id = ?2, priority = ?3, deadline = ?4, reminder = ?5 WHERE id = ?6",
            params![title, project_id, priority, deadline, reminder, id],
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
            "SELECT id, title, description, project_id, status, priority, deadline, reminder, completed FROM tasks \
             WHERE reminder IS NOT NULL AND reminder <= ?1 AND completed = 0 \
             ORDER BY reminder ASC",
        )?;
        let rows = stmt.query_map(params![now], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                project_id: row.get(3)?,
                status: status_from_db(row.get(4)?),
                priority: row.get::<_, Option<i64>>(5)?.unwrap_or(1),
                deadline: row.get(6)?,
                reminder: row.get(7)?,
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

        let id = db
            .add_task("Test", None, None, 1, Some(123), Some(120))
            .unwrap();
        let tasks = db.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id);

        db.mark_done(id).unwrap();
        let tasks = db.list_tasks().unwrap();
        assert!(matches!(tasks[0].status, Status::Done));

        db.delete_task(id).unwrap();
        let tasks = db.list_tasks().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn due_reminders_filters() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let db = Db::new_with_path(&path).unwrap();

        db.add_task("Soon", None, None, 1, None, Some(100)).unwrap();
        db.add_task("Later", None, None, 1, None, Some(200))
            .unwrap();

        let due = db.due_reminders(150).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].title, "Soon");
    }
}
