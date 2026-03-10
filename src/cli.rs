use crate::db::Db;
use crate::task::{
    format_datetime, normalize_priority, parse_datetime_local, priority_label, status_label,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "todo", version, about = "Lightweight CLI todo manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a task
    Add {
        title: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, value_name = "LEVEL", default_value = "normal")]
        priority: String,
        #[arg(long)]
        deadline: Option<String>,
        #[arg(long, value_name = "DATETIME")]
        remind: Option<String>,
    },
    /// List tasks
    List,
    /// Mark task as done
    Done { id: i64 },
    /// Delete a task
    Delete { id: i64 },
    /// Send reminder notifications for due tasks
    Notify {
        #[arg(long, value_name = "MINUTES", default_value_t = 15)]
        snooze_minutes: i64,
    },
    /// Launch interactive UI
    Ui,
    /// Manage projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// Assign or clear a project for a task
    SetProject {
        id: i64,
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Add a project
    Add { name: String },
    /// List projects
    List,
    /// Rename a project
    Rename { id: i64, name: String },
    /// Delete a project (tasks become unassigned)
    Delete { id: i64 },
}

pub fn run_cli(cli: Cli) -> Result<(), String> {
    match cli.command {
        Commands::Add {
            title,
            description,
            project,
            deadline,
            remind,
            priority,
        } => {
            let db = Db::new().map_err(|e| e.to_string())?;
            let project_id = match project {
                Some(name) => match db.project_id_by_name(&name).map_err(|e| e.to_string())? {
                    Some(id) => Some(id),
                    None => return Err(format!("No project named '{}' (create it first)", name)),
                },
                None => None,
            };
            let deadline_ts = parse_optional_datetime(deadline)?;
            let reminder_ts = parse_optional_datetime(remind)?;
            let priority_value = parse_priority(&priority)?;
            let id = db
                .add_task(
                    &title,
                    description.as_deref(),
                    project_id,
                    priority_value,
                    deadline_ts,
                    reminder_ts,
                )
                .map_err(|e| e.to_string())?;
            println!("Added task {}", id);
            Ok(())
        }
        Commands::List => {
            let db = Db::new().map_err(|e| e.to_string())?;
            let tasks = db.list_tasks().map_err(|e| e.to_string())?;
            print_tasks(&tasks);
            Ok(())
        }
        Commands::Done { id } => {
            let db = Db::new().map_err(|e| e.to_string())?;
            let updated = db.mark_done(id).map_err(|e| e.to_string())?;
            if updated == 0 {
                return Err(format!("No task with id {}", id));
            }
            println!("Marked task {} as done", id);
            Ok(())
        }
        Commands::Delete { id } => {
            let db = Db::new().map_err(|e| e.to_string())?;
            let deleted = db.delete_task(id).map_err(|e| e.to_string())?;
            if deleted == 0 {
                return Err(format!("No task with id {}", id));
            }
            println!("Deleted task {}", id);
            Ok(())
        }
        Commands::Notify { snooze_minutes } => {
            crate::notify::run_notify(snooze_minutes).map_err(|e| e.to_string())
        }
        Commands::Ui => crate::tui::run_tui().map_err(|e| e.to_string()),
        Commands::Project { command } => {
            let mut db = Db::new().map_err(|e| e.to_string())?;
            match command {
                ProjectCommands::Add { name } => {
                    let id = db.create_project(&name).map_err(|e| e.to_string())?;
                    println!("Added project {} ({})", name, id);
                }
                ProjectCommands::List => {
                    let projects = db.list_projects().map_err(|e| e.to_string())?;
                    println!("ID  Name");
                    println!("--  ----");
                    for project in projects {
                        println!("{:<3} {}", project.id, project.name);
                    }
                }
                ProjectCommands::Rename { id, name } => {
                    let updated = db.rename_project(id, &name).map_err(|e| e.to_string())?;
                    if updated == 0 {
                        return Err(format!("No project with id {}", id));
                    }
                    println!("Renamed project {} to {}", id, name);
                }
                ProjectCommands::Delete { id } => {
                    db.delete_project(id).map_err(|e| e.to_string())?;
                    println!("Deleted project {}", id);
                }
            }
            Ok(())
        }
        Commands::SetProject { id, project } => {
            let db = Db::new().map_err(|e| e.to_string())?;
            let project_id = match project {
                Some(name) => match db.project_id_by_name(&name).map_err(|e| e.to_string())? {
                    Some(id) => Some(id),
                    None => return Err(format!("No project named '{}' (create it first)", name)),
                },
                None => None,
            };
            let updated = db
                .set_task_project(id, project_id)
                .map_err(|e| e.to_string())?;
            if updated == 0 {
                return Err(format!("No task with id {}", id));
            }
            println!("Updated task {}", id);
            Ok(())
        }
    }
}

fn parse_optional_datetime(input: Option<String>) -> Result<Option<i64>, String> {
    match input {
        Some(value) => parse_datetime_local(&value).map(Some),
        None => Ok(None),
    }
}

fn print_tasks(tasks: &[crate::task::Task]) {
    println!(
        "ID  Title                       Priority      Deadline          Reminder          Status"
    );
    println!(
        "--  --------------------------  ------------  ----------------  ----------------  ------"
    );
    for task in tasks {
        let deadline = format_datetime(task.deadline);
        let reminder = format_datetime(task.reminder);
        let status = status_label(task.status);
        let priority = priority_label(task.priority);
        println!(
            "{:<3} {:<26} {:<12} {:<16} {:<16} {:<6}",
            task.id,
            truncate(&task.title, 26),
            priority,
            deadline,
            reminder,
            status
        );
    }
}

fn truncate(input: &str, max: usize) -> String {
    if input.len() <= max {
        return input.to_string();
    }
    if max <= 3 {
        return ".".repeat(max);
    }
    let mut out = input.chars().take(max - 3).collect::<String>();
    out.push_str("...");
    out
}

fn parse_priority(input: &str) -> Result<i64, String> {
    let normalized = input.trim().to_lowercase().replace(['-', '_'], " ");
    let value = match normalized.as_str() {
        "very high" | "veryhigh" | "vh" | "4" => 4,
        "medium high" | "mediumhigh" | "mh" | "3" => 3,
        "high" | "h" | "2" => 2,
        "normal" | "n" | "1" => 1,
        "low" | "l" | "0" => 0,
        _ => {
            return Err(format!(
                "Invalid priority: {} (use very high, medium high, high, normal, low)",
                input
            ))
        }
    };
    Ok(normalize_priority(value))
}
