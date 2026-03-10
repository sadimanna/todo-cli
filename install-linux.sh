#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust (rustup) first." >&2
  exit 1
fi

mkdir -p "$HOME/.local/bin"

(
  cd "$ROOT_DIR"
  cargo build --release
  cp "target/release/todo" "$HOME/.local/bin/"
)

install_systemd() {
  if ! command -v systemctl >/dev/null 2>&1; then
    echo "systemctl not found. Falling back to cron." >&2
    install_cron
    return
  fi

  mkdir -p "$HOME/.config/systemd/user"
  cp "$ROOT_DIR/systemd/todo-reminder.service" "$HOME/.config/systemd/user/"
  cp "$ROOT_DIR/systemd/todo-reminder.timer" "$HOME/.config/systemd/user/"

  systemctl --user daemon-reload
  systemctl --user enable todo-reminder.timer
  systemctl --user start todo-reminder.timer

  echo "Enabled systemd timer todo-reminder.timer"
}

install_cron() {
  if ! command -v crontab >/dev/null 2>&1; then
    echo "crontab not found. Install cron or choose systemd." >&2
    exit 1
  fi

  local cron_line="* * * * * $HOME/.local/bin/todo notify"
  local existing
  existing="$(crontab -l 2>/dev/null || true)"

  if echo "$existing" | grep -Fq "$cron_line"; then
    echo "Cron entry already exists."
  else
    printf "%s\n%s\n" "$existing" "$cron_line" | crontab -
    echo "Installed cron entry for todo reminders."
  fi
}

choose_scheduler() {
  local choice="${1:-}"
  if [ -z "$choice" ] && [ -t 0 ]; then
    echo "Choose scheduler:"
    echo "1) systemd (recommended)"
    echo "2) cron"
    printf "> "
    read -r choice
  fi

  case "$choice" in
    2|cron|CRON)
      install_cron
      ;;
    *)
      install_systemd
      ;;
  esac
}

choose_scheduler "${TODO_SCHEDULER:-}"

echo "Installed todo to $HOME/.local/bin/todo"
