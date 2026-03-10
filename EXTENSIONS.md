# PLAN.md — Cross-Platform Support and New Features

This document summarizes the planned improvements for **todo-cli** in a phased roadmap.

The goals are to:

1. Make the tool **cross-platform** (Linux, macOS, Windows)
2. Add high-value productivity commands (`todo today`, recurring tasks)
3. Use **native OS schedulers optimally** while still allowing **cron as an optional fallback**
4. Ensure reminders are **never permanently missed**
5. Keep the architecture clean and maintainable

The work is divided into **six implementation phases**.

---

# Phase 1 — Cross-Platform Notification and Sound Support

## Objective

Remove Linux-only dependencies and introduce a **platform abstraction layer** so the application can run on:

* Linux
* macOS
* Windows

This phase focuses on:

* notifications
* reminder sounds

---

## Architecture Change

Create a platform module:

```
src/platform/
    mod.rs
    linux.rs
    macos.rs
    windows.rs
```

Expose a unified interface:

```
platform::notify(title, message)
platform::play_sound()
```

The rest of the application calls only this interface.

---

## Linux Implementation

Notifications:

```
notify-send
```

Sound:

```
paplay
```

Example sound file:

```
/usr/share/sounds/freedesktop/stereo/message.oga
```

---

## macOS Implementation

Notifications:

```
osascript -e 'display notification ...'
```

Sound:

```
afplay /System/Library/Sounds/Ping.aiff
```

---

## Windows Implementation

Notifications via PowerShell:

```
New-BurntToastNotification
```

Sound support may be added later.

---

## Result

todo-cli will support **desktop notifications and reminder sounds across operating systems** while keeping platform-specific code isolated.

---

# Phase 2 — Cross-Platform Reminder Scheduling

## Objective

Introduce **scheduler integration** so reminders can run automatically without requiring a persistent background service.

Instead of running a daemon, reminders will be triggered by periodically running:

```
todo notify
```

---

## Scheduler Strategy

Use the **native OS scheduler by default**, with **cron available as an optional fallback** on Linux and macOS.

| OS      | Primary Scheduler | Optional |
| ------- | ----------------- | -------- |
| Linux   | systemd timers    | cron     |
| macOS   | launchd           | cron     |
| Windows | Task Scheduler    | —        |

---

## Scheduler Behavior

Schedulers will run the command:

```
todo notify
```

every minute.

The application will:

1. check for reminders due
2. trigger notifications
3. play sound
4. mark reminders as triggered

---

## Linux Implementation

### Default: systemd timers

Example timer configuration:

```
todo-reminder.timer
```

```
[Unit]
Description=Run todo reminder checks

[Timer]
OnCalendar=*:0/1
Persistent=true

[Install]
WantedBy=timers.target
```

Important option:

```
Persistent=true
```

This ensures **missed scheduler runs execute immediately after reboot**.

---

### Optional: cron

Users may choose cron during installation.

Example entry:

```
* * * * * todo notify
```

---

## macOS Implementation

### Default: launchd

Launch agent example:

```
~/Library/LaunchAgents/com.todo.reminder.plist
```

Example configuration:

```
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.todo.reminder</string>

  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/todo</string>
    <string>notify</string>
  </array>

  <key>StartInterval</key>
  <integer>60</integer>
</dict>
</plist>
```

---

### Optional: cron

Users may choose cron during installation.

Example:

```
* * * * * todo notify
```

---

## Windows Implementation

Use **Windows Task Scheduler**.

Example task configuration:

Program:

```
todo.exe
```

Arguments:

```
notify
```

Trigger:

```
repeat every 1 minute
```

Windows will always use **Task Scheduler**.

---

## Installation Behavior

The installation script detects the operating system.

Linux:

```
choose scheduler:
1. systemd (recommended)
2. cron
```

macOS:

```
choose scheduler:
1. launchd (recommended)
2. cron
```

Windows:

```
Task Scheduler configured automatically
```

---

## Benefits

* avoids running a persistent daemon
* minimal CPU usage
* integrates with native OS scheduling systems
* supports missed scheduler runs after reboot
* maintains cross-platform compatibility

---

# Phase 3 — Reliable Reminder Delivery (Missed Reminder Catch-Up)

## Objective

Ensure reminders are **never permanently missed**, even if:

* the laptop was turned off
* the system was asleep
* the scheduler did not run at the reminder time

---

## Design Principle

Instead of triggering reminders only when:

```
reminder == current_time
```

the system checks:

```
reminder <= now
AND reminder_triggered = false
```

This allows **missed reminders to trigger once the system becomes active again**.

---

## Database Change

Add a flag to track reminder delivery.

```
ALTER TABLE tasks
ADD COLUMN reminder_triggered BOOLEAN DEFAULT 0;
```

---

## Reminder Query

```
SELECT id, title, reminder
FROM tasks
WHERE reminder IS NOT NULL
AND reminder <= CURRENT_TIMESTAMP
AND reminder_triggered = 0;
```

---

## Reminder Trigger Flow

```
scheduler runs todo notify
      ↓
query due reminders
      ↓
trigger notification + sound
      ↓
mark reminder_triggered = true
```

Example update:

```
UPDATE tasks
SET reminder_triggered = 1
WHERE id = ?;
```

---

# Phase 4 — Add `todo today` Command

## Objective

Introduce a command that surfaces **tasks requiring attention today**.

Most users do not want to see the entire task list.
They want to know **what needs to be done now**.

---

## CLI Usage

```
todo today
```

Example output:

```
Overdue
-------
[!] Write SSL experiment code (due Mar 9)

Today
-----
[ ] Submit conference abstract (due today)

Upcoming
--------
[ ] Prepare slides (due Mar 12)
```

---

## Categories

Tasks are grouped into three categories.

| Category | Condition                 |
| -------- | ------------------------- |
| Overdue  | deadline < today          |
| Today    | deadline = today          |
| Upcoming | deadline ≤ today + 3 days |

---

## SQL Query

```
SELECT
    id,
    title,
    deadline
FROM tasks
WHERE deadline IS NOT NULL
AND deadline <= date('now', '+3 days')
ORDER BY deadline;
```

---

## Classification Logic

```
if deadline < today
    → overdue

if deadline == today
    → today

if deadline > today
    → upcoming
```

---

Similarly, do one for 'todo week` (agenda view) which shows the tasks pending for the coming week in a similar way to 'todo today'

# Phase 5 — Recurring Tasks

## Objective

Support tasks that **automatically repeat after completion**.

Recurring tasks are useful for:

* daily routines
* weekly meetings
* periodic maintenance tasks

Example:

```
todo add "Backup research data" --repeat weekly
```

---

## Example Recurring Tasks

| Task                 | Frequency |
| -------------------- | --------- |
| Backup research data | weekly    |
| Review experiments   | daily     |
| Team meeting         | weekly    |
| Pay rent             | monthly   |

---

## Database Changes

Add recurrence fields to the tasks table.

```
ALTER TABLE tasks
ADD COLUMN recurrence TEXT;
```

Example values:

```
daily
weekly
monthly
yearly
```

---

## Completion Behavior

When a recurring task is marked as completed:

```
todo done 5
```

The system performs:

```
mark task completed
      ↓
create new task with next scheduled date
```

Example:

```
Weekly meeting
completed → new task created for next week
```

---

## Recurrence Logic

Example logic:

| Recurrence | Next task |
| ---------- | --------- |
| daily      | +1 day    |
| weekly     | +7 days   |
| monthly    | +1 month  |

---

# Phase 6 — UI Integration (Optional)

Integrate new functionality into the terminal UI.

Possible additions:

Shortcut for today's tasks:

```
t → show today tasks
```

Recurring tasks indicator:

```
↻ icon next to recurring tasks
```

---

# Expected Outcome

After completing these phases, todo-cli will provide:

* cross-platform notifications
* cross-platform reminder scheduling
* native OS scheduler integration
* optional cron scheduling
* reliable reminder delivery
* reminder sounds
* `todo today` productivity command
* recurring task automation
* a cleaner architecture for future features

These changes make the tool **portable, reliable, and more powerful as a daily productivity tool**.
