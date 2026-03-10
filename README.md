# Todo CLI

A lightweight Linux todo manager written in Rust.

Features:
- CLI task management (add, list, done, delete)
- Task priorities (Very High, Medium High, High, Normal, Low)
- SQLite persistence
- Deadlines and reminders
- Desktop notifications via libnotify
- systemd user timer for reminder checks
- Interactive TUI (ratatui + crossterm) with search and calendar picker

## Requirements

- Rust toolchain (cargo)
- SQLite development library
  - Debian/Ubuntu: `sudo apt-get install -y libsqlite3-dev`
- libnotify runtime and development files
  - Debian/Ubuntu: `sudo apt-get install -y libnotify-bin libnotify-dev`

## Build

From the project directory:

```bash
source "$HOME/.cargo/env"
cargo build --release
```

Note:
- If you used `rustup`, `cargo` may not be on your PATH in new shells until you run `source "$HOME/.cargo/env"` or restart the terminal.
- The correct release command is `cargo build --release` (with `--`).

Binary output:

```
target/release/todo
```

Optional local install:

```bash
cp target/release/todo ~/.local/bin/
```

## One-Step Install (Recommended)

This builds the binary, installs it to `~/.local/bin/`, and installs/enables the systemd reminder timer.

```bash
./install.sh
```

## Run (CLI)

From source without install:

```bash
cargo run -- add "Write paper" --deadline "2026-03-15 18:00" --remind "2026-03-15 16:00"
cargo run -- add "Write paper" --priority "very high"
cargo run -- list
cargo run -- done 3
cargo run -- delete 2
```

If installed:

```bash
todo add "Write paper" --deadline "2026-03-15 18:00" --remind "2026-03-15 16:00"
todo add "Write paper" --priority "very high"
todo list
todo done 3
todo delete 2
```

Notes:
- Datetime format: `YYYY-MM-DD HH:MM` (local time)
- DB location: `~/.todo/tasks.db`
- Override DB path for testing: `TODO_DB_PATH=/path/to/tasks.db`
- Priority values: `very high`, `medium high`, `high`, `normal`, `low` (also accepts `vh`, `mh`, `h`, `n`, `l`)

## Run (TUI)

From source:

```bash
cargo run -- ui
```

If installed:

```bash
todo ui
```

TUI key bindings (normal mode):
- `q` quit
- `a` add task
- `e` edit task
- `x` mark done
- `d` delete task
- `/` search
- `Up/Down` navigate

Add task mode:
- `Tab` switch fields
- `Left/Right` move cursor within text fields
- `Enter` save
- `Esc` cancel
- `Ctrl+O` open calendar picker (for deadline/reminder fields)
- `+` / `-` change priority (when Priority field is selected)

Calendar picker:
- Arrow keys move day/week
- `PgUp/PgDn` change month
- `Enter` select
- `Esc` cancel

## Notifications

Run reminders manually:

```bash
cargo run -- notify
```

Or if installed:

```bash
todo notify
```

By default, reminders are snoozed for 15 minutes after each notification. The popup includes a Snooze button (if supported by your notification server).

You can customize snooze duration (minutes):

```bash
todo notify --snooze-minutes 30
```

## systemd User Timer

Install unit files:

```bash
mkdir -p ~/.config/systemd/user
cp systemd/todo-reminder.service ~/.config/systemd/user/
cp systemd/todo-reminder.timer ~/.config/systemd/user/
```

Enable timer:

```bash
systemctl --user daemon-reload
systemctl --user enable todo-reminder.timer
systemctl --user start todo-reminder.timer
```

This triggers `todo notify` every minute.
