use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table};
use ratatui::Frame;

use chrono::Datelike;

use crate::task::{format_datetime, priority_label};

use super::app::{AddField, App, Mode};
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
    draw_tasks(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);

    match app.mode {
        Mode::AddTask => draw_add_popup(frame, app),
        Mode::Calendar => draw_calendar_popup(frame, app),
        Mode::Time => {
            draw_calendar_popup(frame, app);
            draw_time_popup(frame, app);
        }
        _ => {}
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = match app.mode {
        Mode::Search => format!("Todo (search: {})", app.search_query),
        Mode::AddTask => "Todo (add task)".to_string(),
        Mode::Calendar => "Todo (calendar)".to_string(),
        Mode::Time => "Todo (time)".to_string(),
        Mode::Normal => "Todo".to_string(),
    };
    let block = Block::default().title(title).borders(Borders::ALL);
    frame.render_widget(block, area);
}

fn draw_tasks(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        "ID", "Task", "Priority", "Deadline", "Reminder", "Status",
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let visible = app.visible_indices();
    let rows: Vec<Row> = visible
        .iter()
        .filter_map(|idx| app.tasks.get(*idx))
        .map(|task| {
            let style = priority_style(task.priority);
            Row::new(vec![
                task.id.to_string(),
                task.title.clone(),
                priority_label(task.priority).to_string(),
                format_datetime(task.deadline),
                format_datetime(task.reminder),
                if task.completed {
                    "Done".to_string()
                } else {
                    "Pending".to_string()
                },
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(38),
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL))
    .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

    let mut state = table_state(app.selected);
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.mode {
        Mode::Normal => "a:add e:edit d:delete x:done /:search q:quit",
        Mode::Search => "type to search, enter/esc to exit",
        Mode::AddTask => {
            "tab:next field left/right:move enter:save esc:cancel ctrl+o:calendar +/- priority"
        }
        Mode::Calendar => "arrows:move pgup/pgdn:month enter:time esc:cancel",
        Mode::Time => "left/right:field up/down:change pgup/pgdn:+/-5 enter:select esc:back",
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
        AddField::Deadline => (
            1,
            "Deadline (YYYY-MM-DD HH:MM): ".len(),
            app.add_form.cursor_deadline,
        ),
        AddField::Reminder => (
            2,
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

fn table_state(selected: usize) -> ratatui::widgets::TableState {
    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(selected));
    state
}
