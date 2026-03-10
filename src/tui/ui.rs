use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use chrono::Datelike;

use crate::task::priority_label;

use super::app::{AddField, App, Focus, Mode};
use super::calendar::{days_in_month, month_name};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(frame.size());

    draw_header(frame, chunks[0], app);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(chunks[1]);
    draw_projects(frame, body[0], app);
    draw_board(frame, body[1], app);
    draw_footer(frame, chunks[2], app);

    match app.mode {
        Mode::AddTask => draw_add_popup(frame, app),
        Mode::Calendar => draw_calendar_popup(frame, app),
        Mode::Time => {
            draw_calendar_popup(frame, app);
            draw_time_popup(frame, app);
        }
        Mode::Project => draw_project_popup(frame, app),
        _ => {}
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = match app.mode {
        Mode::Search => format!("Todo ({}) (search: {})", app.active_project_name(), app.search_query),
        Mode::AddTask => "Todo (add task)".to_string(),
        Mode::Calendar => "Todo (calendar)".to_string(),
        Mode::Time => "Todo (time)".to_string(),
        Mode::Project => "Todo (project)".to_string(),
        Mode::Normal => format!("Todo ({})", app.active_project_name()),
    };
    let block = Block::default().title(title).borders(Borders::ALL);
    frame.render_widget(block, area);
}

fn draw_projects(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .map(|project| ListItem::new(project.name.clone()))
        .collect();

    let border_style = if app.focus == Focus::Projects {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let highlight_style = if app.focus == Focus::Projects {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Projects")
                .border_style(border_style),
        )
        .highlight_style(highlight_style);

    let mut state = project_state(app.selected_project, app.projects.len());
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_board(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let indices = app.board_indices();
    let titles = ["TODO", "IN PROGRESS", "DONE"];

    for (col_idx, col_area) in columns.iter().enumerate() {
        let items: Vec<ListItem> = indices[col_idx]
            .iter()
            .filter_map(|idx| app.tasks.get(*idx))
            .map(|task| {
                let title = format!("{} {}", task.id, task.title);
                ListItem::new(title).style(priority_style(task.priority))
            })
            .collect();

        let border_style = if app.focus == Focus::Tasks && app.board.column == col_idx {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let highlight_style = if app.focus == Focus::Tasks && app.board.column == col_idx {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(titles[col_idx])
                    .border_style(border_style),
            )
            .highlight_style(highlight_style);

        let mut state = list_state(
            app.board.row,
            app.focus == Focus::Tasks && app.board.column == col_idx,
            indices[col_idx].len(),
        );
        frame.render_stateful_widget(list, *col_area, &mut state);
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.mode {
        Mode::Normal => match app.focus {
            Focus::Projects => "tab:focus a:add e:edit d:delete /:search q:quit",
            Focus::Tasks => {
                "tab:focus a:add e:edit d:delete x:done /:search left/right:column h/l:move q:quit"
            }
        },
        Mode::Search => "type to search, enter/esc to exit",
        Mode::AddTask => {
            "tab:next field left/right:move enter:save esc:cancel ctrl+o:calendar +/- priority"
        }
        Mode::Calendar => "arrows:move pgup/pgdn:month enter:time esc:cancel",
        Mode::Time => "left/right:field up/down:change pgup/pgdn:+/-5 enter:select esc:back",
        Mode::Project => "enter:save esc:cancel left/right:move",
    };
    let status = if app.status.is_empty() {
        help
    } else {
        &app.status
    };
    let block = Block::default().borders(Borders::ALL);
    let paragraph = Paragraph::new(Line::from(status));
    frame.render_widget(block, area);
    frame.render_widget(
        paragraph,
        area.inner(ratatui::layout::Margin {
            vertical: 0,
            horizontal: 1,
        }),
    );
}

fn draw_add_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 40, frame.size());
    frame.render_widget(Clear, area);

    let title = if app.edit_id.is_some() {
        "Edit Task"
    } else {
        "Add Task"
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    let lines = vec![
        line_with_label(
            "Title",
            &app.add_form.title,
            app.add_form.field == AddField::Title,
        ),
        line_with_label(
            "Project",
            app.project_name_by_index(app.add_form.project_index),
            app.add_form.field == AddField::Project,
        ),
        line_with_label(
            "Deadline (YYYY-MM-DD HH:MM)",
            &app.add_form.deadline,
            app.add_form.field == AddField::Deadline,
        ),
        line_with_label(
            "Reminder (YYYY-MM-DD HH:MM)",
            &app.add_form.reminder,
            app.add_form.field == AddField::Reminder,
        ),
        line_with_label(
            "Priority",
            priority_label(app.add_form.priority),
            app.add_form.field == AddField::Priority,
        ),
    ];
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);

    if let Some((x, y)) = cursor_position_add(app, inner) {
        frame.set_cursor(x, y);
    }
}

fn draw_project_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    let title = if app.project_form.edit_id.is_some() {
        "Edit Project"
    } else {
        "Add Project"
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    let line = line_with_label("Name", &app.project_form.name, true);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, inner);

    let x = inner.x + "Name: ".len() as u16 + app.project_form.cursor as u16;
    let y = inner.y;
    frame.set_cursor(x, y);
}

fn draw_calendar_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, frame.size());
    frame.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title("Select Date");
    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });
    let mut lines: Vec<Line> = Vec::new();

    let title = format!(
        "{} {}",
        month_name(app.calendar.selected_month),
        app.calendar.selected_year
    );
    lines.push(Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    let header = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    let header_line: String = header.iter().map(|label| format!("{:<4}", label)).collect();
    lines.push(Line::from(header_line));

    let first_day =
        chrono::NaiveDate::from_ymd_opt(app.calendar.selected_year, app.calendar.selected_month, 1)
            .unwrap();
    let offset = first_day.weekday().num_days_from_monday() as usize;
    let days = days_in_month(app.calendar.selected_year, app.calendar.selected_month) as usize;

    let mut cells: Vec<String> = Vec::new();
    for _ in 0..offset {
        cells.push("    ".to_string());
    }
    for day in 1..=days {
        let label = if day as u32 == app.calendar.selected_day {
            format!("[{:>2}]", day)
        } else {
            format!(" {:>2} ", day)
        };
        cells.push(label);
    }

    let mut row: Vec<String> = Vec::new();
    for cell in cells.into_iter() {
        row.push(cell);
        if row.len() == 7 {
            lines.push(Line::from(row.join("")));
            row = Vec::new();
        }
    }
    if !row.is_empty() {
        while row.len() < 7 {
            row.push("    ".to_string());
        }
        lines.push(Line::from(row.join("")));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn line_with_label(label: &str, value: &str, active: bool) -> Line<'static> {
    let style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(format!("{}: ", label), style),
        Span::raw(value.to_string()),
    ])
}

fn priority_style(priority: i64) -> Style {
    match priority {
        4 => Style::default().fg(Color::Red),
        3 => Style::default().fg(Color::Magenta),
        2 => Style::default().fg(Color::Yellow),
        1 => Style::default().fg(Color::Green),
        0 => Style::default().fg(Color::Blue),
        _ => Style::default(),
    }
}

fn cursor_position_add(app: &App, inner: Rect) -> Option<(u16, u16)> {
    let (row_index, label_len, cursor_pos) = match app.add_form.field {
        AddField::Title => (0, "Title: ".len(), app.add_form.cursor_title),
        AddField::Project => return None,
        AddField::Deadline => (
            2,
            "Deadline (YYYY-MM-DD HH:MM): ".len(),
            app.add_form.cursor_deadline,
        ),
        AddField::Reminder => (
            3,
            "Reminder (YYYY-MM-DD HH:MM): ".len(),
            app.add_form.cursor_reminder,
        ),
        AddField::Priority => return None,
    };
    let x = inner.x + label_len as u16 + cursor_pos as u16;
    let y = inner.y + row_index as u16;
    Some((x, y))
}

fn draw_time_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(30, 20, frame.size());
    frame.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title("Select Time");
    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 2,
    });

    let hour_style = if app.time_picker.field == super::app::TimeField::Hour {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let minute_style = if app.time_picker.field == super::app::TimeField::Minute {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let line = Line::from(vec![
        Span::raw("Time: "),
        Span::styled(format!("{:02}", app.time_picker.hour), hour_style),
        Span::raw(":"),
        Span::styled(format!("{:02}", app.time_picker.minute), minute_style),
    ]);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn project_state(selected: usize, len: usize) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    if len > 0 {
        state.select(Some(selected.min(len - 1)));
    }
    state
}

fn list_state(selected: usize, active: bool, len: usize) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    if active && len > 0 {
        state.select(Some(selected.min(len - 1)));
    }
    state
}
