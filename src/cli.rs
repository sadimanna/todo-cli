use crate::db::{Db, NewTask};
use crate::task::{
    format_datetime, normalize_priority, parse_datetime_local, priority_label, status_label,
};
use chrono::{Duration, Local, NaiveDate, TimeZone};
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
        #[arg(short, long, alias = "desc")]
        description: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, value_name = "LEVEL", default_value = "normal")]
        priority: String,
        #[arg(long)]
        deadline: Option<String>,
        #[arg(long, value_name = "DATETIME")]
        remind: Option<String>,
        #[arg(long, value_name = "FREQ")]
        repeat: Option<String>,
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
    /// Show tasks due today and upcoming (next 3 days)
    Today,
    /// Show agenda for the coming week (next 7 days)
    Week,
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
            repeat,
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
            let recurrence = parse_recurrence(repeat.as_deref())?;
            let id = db
                .add_task(NewTask {
                    title: &title,
                    description: description.as_deref(),
                    project_id,
                    priority: priority_value,
                    deadline: deadline_ts,
                    reminder: reminder_ts,
                    recurrence: recurrence.as_deref(),
                })
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
            let (updated, new_id) = db.complete_task(id).map_err(|e| e.to_string())?;
            if updated == 0 {
                return Err(format!("No task with id {}", id));
            }
            println!("Marked task {} as done", id);
            if let Some(new_id) = new_id {
                println!("Created recurring task {}", new_id);
            }
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
        Commands::Today => run_agenda(3).map_err(|e| e.to_string()),
        Commands::Week => run_agenda(7).map_err(|e| e.to_string()),
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

fn run_agenda(window_days: i64) -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::new()?;
    let today = Local::now().date_naive();
    let end_date = today + Duration::days(window_days);
    let end_ts = end_of_day_ts(end_date)?;
    let tasks = db.tasks_due_before(end_ts)?;

    let mut overdue = Vec::new();
    let mut due_today = Vec::new();
    let mut upcoming = Vec::new();

    for task in tasks {
        if let Some(deadline_ts) = task.deadline {
            if let Some(deadline_date) = deadline_date(deadline_ts) {
                if deadline_date < today {
                    overdue.push(task);
                } else if deadline_date == today {
                    due_today.push(task);
                } else {
                    upcoming.push(task);
                }
            }
        }
    }

    if overdue.is_empty() && due_today.is_empty() && upcoming.is_empty() {
        println!("No tasks due.");
        return Ok(());
    }

    print_agenda_section("Overdue", &overdue, today, true);
    print_agenda_section("Today", &due_today, today, false);
    print_agenda_section("Upcoming", &upcoming, today, false);

    Ok(())
}

fn deadline_date(deadline_ts: i64) -> Option<NaiveDate> {
    Local
        .timestamp_opt(deadline_ts, 0)
        .single()
        .map(|dt| dt.date_naive())
}

fn end_of_day_ts(date: NaiveDate) -> Result<i64, Box<dyn std::error::Error>> {
    let next_day = date + Duration::days(1);
    let naive = next_day
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| "Invalid day boundary".to_string())?;
    let dt = Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
        .or_else(|| Local.from_local_datetime(&naive).latest())
        .ok_or_else(|| "Invalid or ambiguous local time".to_string())?;
    Ok(dt.timestamp() - 1)
}

fn print_agenda_section(title: &str, tasks: &[crate::task::Task], today: NaiveDate, overdue: bool) {
    println!("{}", title);
    println!("{}", "-".repeat(title.len()));
    if tasks.is_empty() {
        println!("(none)\n");
        return;
    }

    for task in tasks {
        let deadline_label = task
            .deadline
            .and_then(deadline_date)
            .map(|date| {
                if date == today {
                    "due today".to_string()
                } else {
                    format!("due {}", date.format("%Y-%m-%d"))
                }
            })
            .unwrap_or_else(|| "no deadline".to_string());
        let marker = if overdue { "[!]" } else { "[ ]" };
        println!("{} {} ({})", marker, task.title, deadline_label);
    }
    println!();
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

fn parse_recurrence(input: Option<&str>) -> Result<Option<String>, String> {
    let value = match input {
        Some(value) => value.trim(),
        None => return Ok(None),
    };
    if value.is_empty() {
        return Ok(None);
    }
    let normalized = value.to_lowercase();
    match normalized.as_str() {
        "daily" | "weekly" | "monthly" | "yearly" => Ok(Some(normalized)),
        _ => Err(format!(
            "Invalid recurrence: {} (use daily, weekly, monthly, yearly)",
            value
        )),
    }
}
