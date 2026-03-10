use crate::db::Db;
use crate::task::Task;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddField {
    #[default]
    Title,
    Deadline,
    Reminder,
    Priority,
}

#[derive(Debug, Clone)]
pub struct AddForm {
    pub title: String,
    pub deadline: String,
    pub reminder: String,
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

pub struct App {
    pub db: Db,
    pub tasks: Vec<Task>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub add_form: AddForm,
    pub edit_id: Option<i64>,
    pub calendar: CalendarState,
    pub calendar_target: Option<CalendarTarget>,
    pub time_picker: TimePicker,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let db = Db::new().map_err(|e| e.to_string())?;
        let tasks = db.list_tasks().map_err(|e| e.to_string())?;
        Ok(Self {
            db,
            tasks,
            selected: 0,
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
            status: String::new(),
            should_quit: false,
        })
    }

    pub fn refresh_tasks(&mut self) -> Result<(), String> {
        self.tasks = self.db.list_tasks().map_err(|e| e.to_string())?;
        if self.selected >= self.visible_indices().len() {
            self.selected = self.visible_indices().len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            return (0..self.tasks.len()).collect();
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, usize)> = self
            .tasks
            .iter()
            .enumerate()
            .filter_map(|(idx, task)| {
                matcher
                    .fuzzy_match(&task.title, &self.search_query)
                    .map(|score| (score, idx))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, idx)| idx).collect()
    }

    pub fn selected_task(&self) -> Option<&Task> {
        let visible = self.visible_indices();
        visible
            .get(self.selected)
            .and_then(|idx| self.tasks.get(*idx))
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    pub fn next(&mut self) {
        let len = self.visible_indices().len();
        if len == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected + 1).min(len.saturating_sub(1));
    }

    pub fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn reset_add_form(&mut self) {
        self.add_form = AddForm::default();
    }
}

impl Default for AddForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            deadline: String::new(),
            reminder: String::new(),
            field: AddField::Title,
            priority: 1,
            cursor_title: 0,
            cursor_deadline: 0,
            cursor_reminder: 0,
        }
    }
}
