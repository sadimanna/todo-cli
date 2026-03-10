use crate::db::Db;
use crate::task::{status_column, Task};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::calendar::{CalendarState, CalendarTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    AddTask,
    Calendar,
    Time,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Projects,
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddField {
    #[default]
    Title,
    Project,
    Deadline,
    Reminder,
    Priority,
}

#[derive(Debug, Clone)]
pub struct AddForm {
    pub title: String,
    pub deadline: String,
    pub reminder: String,
    pub project_index: usize,
    pub field: AddField,
    pub priority: i64,
    pub cursor_title: usize,
    pub cursor_deadline: usize,
    pub cursor_reminder: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeField {
    #[default]
    Hour,
    Minute,
}

#[derive(Debug, Clone)]
pub struct TimePicker {
    pub hour: u32,
    pub minute: u32,
    pub field: TimeField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoardState {
    pub column: usize,
    pub row: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectForm {
    pub name: String,
    pub cursor: usize,
    pub edit_id: Option<i64>,
}

pub struct App {
    pub db: Db,
    pub tasks: Vec<Task>,
    pub board: BoardState,
    pub projects: Vec<ProjectEntry>,
    pub selected_project: usize,
    pub focus: Focus,
    pub mode: Mode,
    pub search_query: String,
    pub add_form: AddForm,
    pub edit_id: Option<i64>,
    pub calendar: CalendarState,
    pub calendar_target: Option<CalendarTarget>,
    pub time_picker: TimePicker,
    pub project_form: ProjectForm,
    pub status: String,
    pub should_quit: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub id: Option<i64>,
    pub name: String,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let db = Db::new().map_err(|e| e.to_string())?;
        let projects = build_project_entries(&db)?;
        let selected_project = 0;
        let tasks = match projects.get(selected_project).and_then(|entry| entry.id) {
            Some(project_id) => db
                .list_tasks_for_project(project_id)
                .map_err(|e| e.to_string())?,
            None => db.list_tasks().map_err(|e| e.to_string())?,
        };
        Ok(Self {
            db,
            tasks,
            board: BoardState::default(),
            projects,
            selected_project,
            focus: Focus::Tasks,
            mode: Mode::Normal,
            search_query: String::new(),
            add_form: AddForm::default(),
            edit_id: None,
            calendar: CalendarState::today(),
            calendar_target: None,
            time_picker: TimePicker {
                hour: 9,
                minute: 0,
                field: TimeField::Hour,
            },
            project_form: ProjectForm::default(),
            status: String::new(),
            should_quit: false,
        })
    }

    pub fn refresh_tasks(&mut self) -> Result<(), String> {
        self.tasks = match self.active_project_id() {
            Some(project_id) => self
                .db
                .list_tasks_for_project(project_id)
                .map_err(|e| e.to_string())?,
            None => self.db.list_tasks().map_err(|e| e.to_string())?,
        };
        self.clamp_board();
        Ok(())
    }

    pub fn active_project_id(&self) -> Option<i64> {
        self.projects
            .get(self.selected_project)
            .and_then(|entry| entry.id)
    }

    pub fn active_project_name(&self) -> &str {
        self.projects
            .get(self.selected_project)
            .map(|entry| entry.name.as_str())
            .unwrap_or("All")
    }

    pub fn project_name_by_index(&self, index: usize) -> &str {
        self.projects
            .get(index)
            .map(|entry| entry.name.as_str())
            .unwrap_or("All")
    }

    pub fn project_id_by_index(&self, index: usize) -> Option<i64> {
        self.projects.get(index).and_then(|entry| entry.id)
    }

    pub fn project_index_by_id(&self, id: Option<i64>) -> usize {
        match id {
            Some(project_id) => self
                .projects
                .iter()
                .position(|entry| entry.id == Some(project_id))
                .unwrap_or(0),
            None => 0,
        }
    }

    pub fn select_project_by_id(&mut self, id: i64) {
        if let Some(index) = self.projects.iter().position(|entry| entry.id == Some(id)) {
            self.selected_project = index;
        }
    }

    pub fn next_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        self.selected_project = (self.selected_project + 1).min(self.projects.len() - 1);
        self.board.row = 0;
        let _ = self.refresh_tasks();
    }

    pub fn previous_project(&mut self) {
        if self.selected_project > 0 {
            self.selected_project -= 1;
            self.board.row = 0;
            let _ = self.refresh_tasks();
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Projects => Focus::Tasks,
            Focus::Tasks => Focus::Projects,
        };
    }

    pub fn refresh_projects(&mut self) -> Result<(), String> {
        self.projects = build_project_entries(&self.db)?;
        if self.selected_project >= self.projects.len() {
            self.selected_project = self.projects.len().saturating_sub(1);
        }
        if self.add_form.project_index >= self.projects.len() {
            self.add_form.project_index = self.projects.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn selected_task(&self) -> Option<&Task> {
        let columns = self.board_indices();
        let column = self.board.column.min(columns.len().saturating_sub(1));
        columns
            .get(column)
            .and_then(|col| col.get(self.board.row))
            .and_then(|idx| self.tasks.get(*idx))
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    pub fn next(&mut self) {
        let len = self.board_column_len(self.board.column);
        if len == 0 {
            self.board.row = 0;
            return;
        }
        self.board.row = (self.board.row + 1).min(len.saturating_sub(1));
    }

    pub fn previous(&mut self) {
        if self.board.row > 0 {
            self.board.row -= 1;
        }
    }

    pub fn reset_add_form(&mut self) {
        self.add_form = AddForm::default();
        self.add_form.project_index = self.selected_project;
    }

    pub fn move_column(&mut self, delta: i32) {
        let mut next = self.board.column as i32 + delta;
        next = next.clamp(0, 2);
        self.board.column = next as usize;
        self.clamp_board();
    }

    pub fn board_indices(&self) -> [Vec<usize>; 3] {
        let mut cols = [Vec::new(), Vec::new(), Vec::new()];
        let matcher = SkimMatcherV2::default();
        for (idx, task) in self.tasks.iter().enumerate() {
            if !self.search_query.is_empty()
                && matcher
                    .fuzzy_match(&task.title, &self.search_query)
                    .is_none()
            {
                continue;
            }
            let column = status_column(task.status);
            cols[column].push(idx);
        }
        cols
    }

    pub fn board_column_len(&self, column: usize) -> usize {
        let cols = self.board_indices();
        cols.get(column).map(|col| col.len()).unwrap_or(0)
    }

    fn clamp_board(&mut self) {
        let len = self.board_column_len(self.board.column);
        if len == 0 {
            self.board.row = 0;
        } else if self.board.row >= len {
            self.board.row = len.saturating_sub(1);
        }
    }
}

fn build_project_entries(db: &Db) -> Result<Vec<ProjectEntry>, String> {
    let mut entries = Vec::new();
    let projects = db.list_projects().map_err(|e| e.to_string())?;
    for project in projects {
        entries.push(ProjectEntry {
            id: Some(project.id),
            name: project.name,
        });
    }
    Ok(entries)
}

impl Default for AddForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            deadline: String::new(),
            reminder: String::new(),
            project_index: 0,
            field: AddField::Title,
            priority: 1,
            cursor_title: 0,
            cursor_deadline: 0,
            cursor_reminder: 0,
        }
    }
}
