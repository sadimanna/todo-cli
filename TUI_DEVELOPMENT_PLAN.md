# Todo CLI — UI Development Plan

This document describes the staged development of the terminal UI.

The UI will be implemented incrementally to reduce complexity and ensure a
stable foundation.

Technologies used:

- ratatui
- crossterm
- SQLite

The UI evolves in three phases:

Phase 1 — Single List View  
Phase 2 — Project Sidebar  
Phase 3 — Kanban Board  

Each phase builds on the previous one.

---

# Phase 1 — Single List View

## Objective

Create a minimal interactive UI showing all tasks in a single list.

Example UI:

+---------------------------------------------------+
| Tasks                                             |
+---------------------------------------------------+
| > Write SSL experiment code                      |
|   Read contrastive learning paper                |
|   Buy groceries                                  |
|   Submit reimbursement                           |
+---------------------------------------------------+
| a:add  x:done  d:delete  /:search  q:quit        |
+---------------------------------------------------+

---

## Features

Task navigation  
Task completion  
Task deletion  
Basic search  

---

## Keyboard Controls

Up / Down → navigate tasks  
Enter → open task details  
x → mark task complete  
d → delete task  
a → add task  
/ → search tasks  
q → quit  

---

## Implementation Steps

### 1. Build basic TUI skeleton

Create files:

src/tui/
    app.rs
    ui.rs
    events.rs

---

### 2. Define App state

Example:

struct App {
    tasks: Vec<Task>,
    selected: usize,
}

---

### 3. Implement rendering

Render a list widget.

Use:

ratatui::widgets::List

---

### 4. Implement navigation

Update selected index on key events.

---

### 5. Integrate database

Load tasks on startup.

Reload tasks when:

task added  
task deleted  
task completed  

---

# Phase 2 — Project Sidebar

## Objective

Introduce project grouping with a sidebar.

Example UI:

+-----------+--------------------------------------+
| Projects  | Tasks                                |
+-----------+--------------------------------------+
| > research| Write SSL experiment code           |
|   personal| Read contrastive learning paper     |
|   errands | Buy groceries                       |
|           | Submit reimbursement                |
+-----------+--------------------------------------+

Selecting a project filters tasks.

---

## Features

Project grouping  
Project navigation  
Filtered task view  

---

## Keyboard Controls

Tab → switch focus between panels

Project panel:

Up / Down → change project  
Enter → select project  

Task panel:

Up / Down → navigate tasks  

---

## Implementation Steps

### 1. Add project state

struct App {
    projects: Vec<Project>,
    selected_project: usize,
    tasks: Vec<Task>,
}

---

### 2. Modify layout

Split screen horizontally.

Projects panel (25%)
Tasks panel (75%)

Use:

ratatui Layout

---

### 3. Filter tasks by project

SQL example:

SELECT * FROM tasks
WHERE project_id = ?

---

### 4. Add focus state

enum Focus {
    Projects,
    Tasks
}

---

### 5. Implement panel switching

Tab key switches focus.

---

# Phase 3 — Kanban Board

## Objective

Replace the single task list with a Kanban board.

Example UI:

+-----------+---------------------------------------------+
| Projects  | TODO | IN PROGRESS | DONE                   |
+-----------+---------------------------------------------+
| research  | task1 | task4 | task7                       |
|           | task2 | task5 | task8                       |
|           | task3 |       |                             |
+-----------+---------------------------------------------+

Each column represents a workflow state.

---

## Workflow States

TODO  
IN_PROGRESS  
DONE  

Tasks move between columns.

---

## Keyboard Controls

Left / Right → change column  
Up / Down → change task  
h → move task left  
l → move task right  

---

## Implementation Steps

### 1. Group tasks by status

Example:

Vec<Task> todo
Vec<Task> in_progress
Vec<Task> done

---

### 2. Split UI into columns

Use horizontal layout.

Columns:

TODO  
IN_PROGRESS  
DONE  

Each column renders a List widget.

---

### 3. Track selection

struct BoardState {
    column: usize,
    row: usize,
}

---

### 4. Implement task movement

Pressing `l` moves a task forward.

Example:

TODO → IN_PROGRESS

Database update:

UPDATE tasks
SET status='IN_PROGRESS'
WHERE id=?

---

### 5. Highlight selected task

Use ListState.

---

# Database Schema

The schema is designed to support:

projects  
kanban workflows  
reminders  
future tags  

without major migrations.

---

## Projects Table

CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

---

## Tasks Table

CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    title TEXT NOT NULL,
    description TEXT,

    project_id INTEGER,

    status TEXT DEFAULT 'TODO',

    deadline DATETIME,
    reminder DATETIME,

    completed BOOLEAN DEFAULT 0,

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(project_id) REFERENCES projects(id)
);

---

## Optional Tags (future)

CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE
);

CREATE TABLE task_tags (
    task_id INTEGER,
    tag_id INTEGER,

    PRIMARY KEY(task_id, tag_id)
);

---

# Example Data

Projects:

1 research  
2 personal  

Tasks:

id | title | project_id | status
---|------|-----------|------
1 | Write SSL experiment | 1 | TODO
2 | Read SimCLR paper | 1 | IN_PROGRESS
3 | Buy groceries | 2 | DONE

---

# Query Examples

Load projects:

SELECT * FROM projects;

---

Load tasks for project:

SELECT *
FROM tasks
WHERE project_id = ?

---

Load kanban columns:

SELECT *
FROM tasks
WHERE project_id = ?
AND status = 'TODO';

---

# Future Extensions

This schema supports future features.

Examples:

tags  
recurring tasks  
priority levels  
task dependencies  

Example future column:

ALTER TABLE tasks ADD COLUMN priority INTEGER;

---

# Development Strategy

Recommended order:

1. Implement CLI task management
2. Implement database schema
3. Implement Phase 1 UI
4. Add project grouping
5. Add Kanban board

Do not start with Kanban immediately.

---

# Expected Result

The final UI will resemble tools like:

- lazygit
- k9s
- htop

while remaining lightweight and keyboard-driven.

Binary size: ~4–6 MB  
Memory usage: ~5–10 MB  
Idle CPU usage: ~0%
