use chrono::{Local, NaiveTime, TimeZone, Timelike};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::task::{normalize_priority, parse_datetime_local, status_column, status_from_column};

use super::app::{AddField, App, Focus, Mode, TimeField};
use super::calendar::CalendarTarget;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::Search => handle_search(app, key),
        Mode::AddTask => handle_add(app, key),
        Mode::Calendar => handle_calendar(app, key),
        Mode::Time => handle_time(app, key),
        Mode::Project => handle_project(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Down => match app.focus {
            Focus::Projects => app.next_project(),
            Focus::Tasks => app.next(),
        },
        KeyCode::Up => match app.focus {
            Focus::Projects => app.previous_project(),
            Focus::Tasks => app.previous(),
        },
        KeyCode::Left => {
            if app.focus == Focus::Tasks {
                app.move_column(-1);
            }
        }
        KeyCode::Right => {
            if app.focus == Focus::Tasks {
                app.move_column(1);
            }
        }
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Enter => {
            if app.focus == Focus::Projects {
                let _ = app.refresh_tasks();
                app.board.row = 0;
            }
        }
        KeyCode::Char('a') => {
            if app.focus == Focus::Projects {
                start_project_add(app);
            } else {
                app.reset_add_form();
                app.edit_id = None;
                app.mode = Mode::AddTask;
                app.set_status("");
            }
        }
        KeyCode::Char('e') => {
            if app.focus == Focus::Projects {
                start_project_edit(app);
                return;
            }
            let snapshot = app.selected_task().map(|task| {
                (
                    task.id,
                    task.title.clone(),
                    task.project_id,
                    task.priority,
                    task.deadline,
                    task.reminder,
                )
            });
            if let Some((id, title, project_id, priority, deadline, reminder)) = snapshot {
                app.edit_id = Some(id);
                app.add_form.title = title;
                app.add_form.deadline = format_datetime_input(deadline);
                app.add_form.reminder = format_datetime_input(reminder);
                app.add_form.priority = priority;
                app.add_form.project_index = app.project_index_by_id(project_id);
                app.add_form.cursor_title = app.add_form.title.chars().count();
                app.add_form.cursor_deadline = app.add_form.deadline.chars().count();
                app.add_form.cursor_reminder = app.add_form.reminder.chars().count();
                app.add_form.field = AddField::Title;
                app.mode = Mode::AddTask;
                app.set_status("");
            }
        }
        KeyCode::Char('d') => {
            if app.focus == Focus::Projects {
                delete_selected_project(app);
            } else if let Some(task) = app.selected_task() {
                if app.db.delete_task(task.id).is_ok() {
                    let _ = app.refresh_tasks();
                }
            }
        }
        KeyCode::Char('x') => {
            if app.focus == Focus::Tasks {
                if let Some(task) = app.selected_task() {
                    if app.db.mark_done(task.id).is_ok() {
                        let _ = app.refresh_tasks();
                    }
                }
            }
        }
        KeyCode::Char('h') => {
            if app.focus == Focus::Tasks {
                move_selected_task(app, -1);
            }
        }
        KeyCode::Char('l') => {
            if app.focus == Focus::Tasks {
                move_selected_task(app, 1);
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
            app.board.row = 0;
            app.focus = Focus::Tasks;
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.mode = Mode::Normal;
            app.search_query.clear();
            app.board.row = 0;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.board.row = 0;
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            app.search_query.push(c);
            app.board.row = 0;
        }
        KeyCode::Down => app.next(),
        KeyCode::Up => app.previous(),
        _ => {}
    }
}

fn handle_add(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.edit_id = None;
        }
        KeyCode::Tab => {
            app.add_form.field = next_field(app.add_form.field);
            clamp_cursor(app);
        }
        KeyCode::BackTab => {
            app.add_form.field = prev_field(app.add_form.field);
            clamp_cursor(app);
        }
        KeyCode::Enter => {
            if app.add_form.title.trim().is_empty() {
                app.set_status("Title is required");
                return;
            }
            let project_id = app.project_id_by_index(app.add_form.project_index);
            let deadline = if app.add_form.deadline.trim().is_empty() {
                None
            } else {
                match parse_datetime_local(app.add_form.deadline.trim()) {
                    Ok(value) => Some(value),
                    Err(err) => {
                        app.set_status(err);
                        return;
                    }
                }
            };
            let reminder = if app.add_form.reminder.trim().is_empty() {
                None
            } else {
                match parse_datetime_local(app.add_form.reminder.trim()) {
                    Ok(value) => Some(value),
                    Err(err) => {
                        app.set_status(err);
                        return;
                    }
                }
            };
            let priority = normalize_priority(app.add_form.priority);
            if let Some(id) = app.edit_id {
                if let Err(err) = app.db.update_task(
                    id,
                    app.add_form.title.trim(),
                    project_id,
                    priority,
                    deadline,
                    reminder,
                ) {
                    app.set_status(err.to_string());
                    return;
                }
                app.set_status("Task updated");
            } else {
                if let Err(err) = app.db.add_task(
                    app.add_form.title.trim(),
                    None,
                    project_id,
                    priority,
                    deadline,
                    reminder,
                ) {
                    app.set_status(err.to_string());
                    return;
                }
                app.set_status("Task added");
            }
            let _ = app.refresh_tasks();
            app.mode = Mode::Normal;
            app.edit_id = None;
        }
        KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if matches!(app.add_form.field, AddField::Deadline | AddField::Reminder) {
                app.calendar_target = Some(if app.add_form.field == AddField::Deadline {
                    CalendarTarget::Deadline
                } else {
                    CalendarTarget::Reminder
                });
                app.mode = Mode::Calendar;
            }
        }
        KeyCode::Up => {
            app.add_form.field = prev_field(app.add_form.field);
            clamp_cursor(app);
        }
        KeyCode::Down => {
            app.add_form.field = next_field(app.add_form.field);
            clamp_cursor(app);
        }
        KeyCode::Left => {
            if app.add_form.field == AddField::Project {
                cycle_project(app, -1);
            } else if app.add_form.field == AddField::Priority {
                app.add_form.priority = normalize_priority(app.add_form.priority - 1);
            } else {
                move_cursor(app, -1);
            }
        }
        KeyCode::Right => {
            if app.add_form.field == AddField::Project {
                cycle_project(app, 1);
            } else if app.add_form.field == AddField::Priority {
                app.add_form.priority = normalize_priority(app.add_form.priority + 1);
            } else {
                move_cursor(app, 1);
            }
        }
        KeyCode::Char('-') if app.add_form.field == AddField::Priority => {
            app.add_form.priority = normalize_priority(app.add_form.priority - 1);
        }
        KeyCode::Char('+') if app.add_form.field == AddField::Priority => {
            app.add_form.priority = normalize_priority(app.add_form.priority + 1);
        }
        KeyCode::Backspace => {
            if let Some((field, cursor)) = current_field_and_cursor_mut(app) {
                if *cursor > 0 {
                    delete_char(field, *cursor - 1);
                    *cursor -= 1;
                }
            }
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            if let Some((field, cursor)) = current_field_and_cursor_mut(app) {
                insert_char(field, *cursor, ch);
                *cursor += 1;
            }
        }
        _ => {}
    }
}

fn handle_project(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.project_form = super::app::ProjectForm::default();
        }
        KeyCode::Enter => {
            let name = app.project_form.name.trim();
            if name.is_empty() {
                app.set_status("Project name is required");
                return;
            }
            let result = if let Some(id) = app.project_form.edit_id {
                match app.db.rename_project(id, name) {
                    Ok(0) => {
                        app.set_status("No project with that id");
                        return;
                    }
                    Ok(_) => Ok(id),
                    Err(err) => Err(err),
                }
            } else {
                app.db.create_project(name)
            };
            match result {
                Ok(id) => {
                    let _ = app.refresh_projects();
                    app.select_project_by_id(id);
                    let _ = app.refresh_tasks();
                    app.mode = Mode::Normal;
                    app.project_form = super::app::ProjectForm::default();
                    app.set_status("Project saved");
                }
                Err(err) => {
                    app.set_status(err.to_string());
                }
            }
        }
        KeyCode::Left => move_project_cursor(app, -1),
        KeyCode::Right => move_project_cursor(app, 1),
        KeyCode::Backspace => {
            if app.project_form.cursor > 0 {
                delete_project_char(app);
            }
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            insert_project_char(app, ch);
        }
        _ => {}
    }
}

fn handle_calendar(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::AddTask;
        }
        KeyCode::Left => app.calendar.move_days(-1),
        KeyCode::Right => app.calendar.move_days(1),
        KeyCode::Up => app.calendar.move_days(-7),
        KeyCode::Down => app.calendar.move_days(7),
        KeyCode::PageUp => app.calendar.move_months(-1),
        KeyCode::PageDown => app.calendar.move_months(1),
        KeyCode::Enter => {
            if let Some(target) = app.calendar_target {
                let default_time = match target {
                    CalendarTarget::Deadline => NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    CalendarTarget::Reminder => NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                };
                let existing = match target {
                    CalendarTarget::Deadline => &app.add_form.deadline,
                    CalendarTarget::Reminder => &app.add_form.reminder,
                };
                let time = extract_time(existing).unwrap_or(default_time);
                app.time_picker.hour = time.hour();
                app.time_picker.minute = time.minute();
                app.time_picker.field = TimeField::Hour;
                app.mode = Mode::Time;
            }
        }
        _ => {}
    }
}

fn handle_time(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Calendar;
        }
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            app.time_picker.field = match app.time_picker.field {
                TimeField::Hour => TimeField::Minute,
                TimeField::Minute => TimeField::Hour,
            };
        }
        KeyCode::Up => increment_time(app, 1),
        KeyCode::Down => increment_time(app, -1),
        KeyCode::PageUp => increment_time(app, 5),
        KeyCode::PageDown => increment_time(app, -5),
        KeyCode::Enter => {
            if let Some(target) = app.calendar_target {
                let date = app.calendar.selected_date();
                let time = NaiveTime::from_hms_opt(app.time_picker.hour, app.time_picker.minute, 0)
                    .unwrap();
                let updated = format!("{} {}", date.format("%Y-%m-%d"), time.format("%H:%M"));
                match target {
                    CalendarTarget::Deadline => app.add_form.deadline = updated,
                    CalendarTarget::Reminder => app.add_form.reminder = updated,
                }
            }
            app.mode = Mode::AddTask;
        }
        _ => {}
    }
}

fn current_field_and_cursor_mut(app: &mut App) -> Option<(&mut String, &mut usize)> {
    match app.add_form.field {
        AddField::Title => Some((&mut app.add_form.title, &mut app.add_form.cursor_title)),
        AddField::Project => None,
        AddField::Deadline => Some((
            &mut app.add_form.deadline,
            &mut app.add_form.cursor_deadline,
        )),
        AddField::Reminder => Some((
            &mut app.add_form.reminder,
            &mut app.add_form.cursor_reminder,
        )),
        AddField::Priority => None,
    }
}

fn next_field(field: AddField) -> AddField {
    match field {
        AddField::Title => AddField::Project,
        AddField::Project => AddField::Deadline,
        AddField::Deadline => AddField::Reminder,
        AddField::Reminder => AddField::Priority,
        AddField::Priority => AddField::Title,
    }
}

fn prev_field(field: AddField) -> AddField {
    match field {
        AddField::Title => AddField::Priority,
        AddField::Project => AddField::Title,
        AddField::Deadline => AddField::Project,
        AddField::Reminder => AddField::Deadline,
        AddField::Priority => AddField::Reminder,
    }
}

fn clamp_cursor(app: &mut App) {
    let len = match app.add_form.field {
        AddField::Title => app.add_form.title.chars().count(),
        AddField::Project => return,
        AddField::Deadline => app.add_form.deadline.chars().count(),
        AddField::Reminder => app.add_form.reminder.chars().count(),
        AddField::Priority => return,
    };
    match app.add_form.field {
        AddField::Title => app.add_form.cursor_title = app.add_form.cursor_title.min(len),
        AddField::Project => {}
        AddField::Deadline => app.add_form.cursor_deadline = app.add_form.cursor_deadline.min(len),
        AddField::Reminder => app.add_form.cursor_reminder = app.add_form.cursor_reminder.min(len),
        AddField::Priority => {}
    }
}

fn move_cursor(app: &mut App, delta: i32) {
    let (len, cursor) = match app.add_form.field {
        AddField::Title => (
            app.add_form.title.chars().count(),
            &mut app.add_form.cursor_title,
        ),
        AddField::Project => return,
        AddField::Deadline => (
            app.add_form.deadline.chars().count(),
            &mut app.add_form.cursor_deadline,
        ),
        AddField::Reminder => (
            app.add_form.reminder.chars().count(),
            &mut app.add_form.cursor_reminder,
        ),
        AddField::Priority => return,
    };
    let mut next = *cursor as i32 + delta;
    if next < 0 {
        next = 0;
    }
    if next as usize > len {
        next = len as i32;
    }
    *cursor = next as usize;
}

fn cycle_project(app: &mut App, delta: i32) {
    if app.projects.is_empty() {
        app.add_form.project_index = 0;
        return;
    }
    let mut next = app.add_form.project_index as i32 + delta;
    if next < 0 {
        next = 0;
    }
    if next as usize >= app.projects.len() {
        next = app.projects.len().saturating_sub(1) as i32;
    }
    app.add_form.project_index = next as usize;
}

fn insert_char(value: &mut String, cursor: usize, ch: char) {
    let idx = char_to_byte_index(value, cursor);
    value.insert(idx, ch);
}

fn delete_char(value: &mut String, cursor: usize) {
    let idx = char_to_byte_index(value, cursor);
    if idx < value.len() {
        value.remove(idx);
    }
}

fn char_to_byte_index(value: &str, cursor: usize) -> usize {
    value
        .char_indices()
        .nth(cursor)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| value.len())
}

fn extract_time(input: &str) -> Option<NaiveTime> {
    let part = input.split_whitespace().last()?;
    NaiveTime::parse_from_str(part, "%H:%M").ok()
}

fn format_datetime_input(value: Option<i64>) -> String {
    value
        .and_then(|ts| Local.timestamp_opt(ts, 0).single())
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn increment_time(app: &mut App, delta: i32) {
    match app.time_picker.field {
        TimeField::Hour => {
            let mut hour = app.time_picker.hour as i32 + delta;
            while hour < 0 {
                hour += 24;
            }
            app.time_picker.hour = (hour % 24) as u32;
        }
        TimeField::Minute => {
            let mut minute = app.time_picker.minute as i32 + delta;
            while minute < 0 {
                minute += 60;
            }
            app.time_picker.minute = (minute % 60) as u32;
        }
    }
}

fn start_project_add(app: &mut App) {
    app.project_form = super::app::ProjectForm::default();
    app.mode = Mode::Project;
    app.focus = Focus::Projects;
    app.set_status("");
}

fn start_project_edit(app: &mut App) {
    if let Some(entry) = app.projects.get(app.selected_project) {
        if entry.name == "All" {
            app.set_status("Cannot edit All project");
            return;
        }
    } else {
        app.set_status("Cannot edit All projects");
        return;
    }
    if let Some(entry) = app.projects.get(app.selected_project) {
        if let Some(id) = entry.id {
            app.project_form.name = entry.name.clone();
            app.project_form.cursor = app.project_form.name.chars().count();
            app.project_form.edit_id = Some(id);
            app.mode = Mode::Project;
            app.focus = Focus::Projects;
            app.set_status("");
        }
    }
}

fn delete_selected_project(app: &mut App) {
    if let Some(entry) = app.projects.get(app.selected_project) {
        if entry.name == "All" {
            app.set_status("Cannot delete All project");
            return;
        }
    } else {
        app.set_status("Cannot delete All projects");
        return;
    }
    if let Some(entry) = app.projects.get(app.selected_project) {
        if let Some(id) = entry.id {
            if app.db.delete_project(id).is_ok() {
                let _ = app.refresh_projects();
                let _ = app.refresh_tasks();
                app.set_status("Project deleted");
            }
        }
    }
}

fn move_project_cursor(app: &mut App, delta: i32) {
    let len = app.project_form.name.chars().count();
    let mut next = app.project_form.cursor as i32 + delta;
    if next < 0 {
        next = 0;
    }
    if next as usize > len {
        next = len as i32;
    }
    app.project_form.cursor = next as usize;
}

fn insert_project_char(app: &mut App, ch: char) {
    let idx = char_to_byte_index(&app.project_form.name, app.project_form.cursor);
    app.project_form.name.insert(idx, ch);
    app.project_form.cursor += 1;
}

fn delete_project_char(app: &mut App) {
    let idx = char_to_byte_index(&app.project_form.name, app.project_form.cursor - 1);
    if idx < app.project_form.name.len() {
        app.project_form.name.remove(idx);
        app.project_form.cursor -= 1;
    }
}

fn move_selected_task(app: &mut App, delta: i32) {
    let snapshot = app.selected_task().map(|task| (task.id, task.status));
    if let Some((id, status)) = snapshot {
        let mut next_col = status_column(status) as i32 + delta;
        next_col = next_col.clamp(0, 2);
        let next_status = status_from_column(next_col as usize);
        if app.db.set_task_status(id, next_status).is_ok() {
            let _ = app.refresh_tasks();
            app.board.column = next_col as usize;
            let cols = app.board_indices();
            if let Some(pos) = cols[app.board.column]
                .iter()
                .position(|idx| app.tasks.get(*idx).map(|task| task.id) == Some(id))
            {
                app.board.row = pos;
            } else {
                app.board.row = 0;
            }
        }
    }
}
