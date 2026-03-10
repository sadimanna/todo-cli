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

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

(
  cd "$ROOT_DIR"
  cargo build --release
  cp "target/release/todo" "$INSTALL_DIR/"
)

install_launchd() {
  local plist_dir="$HOME/Library/LaunchAgents"
  local plist_path="$plist_dir/com.todo.reminder.plist"

  mkdir -p "$plist_dir"
  cat > "$plist_path" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.todo.reminder</string>
  <key>ProgramArguments</key>
  <array>
    <string>$INSTALL_DIR/todo</string>
    <string>notify</string>
  </array>
  <key>StartInterval</key>
  <integer>60</integer>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
EOF

  launchctl unload "$plist_path" >/dev/null 2>&1 || true
  launchctl load -w "$plist_path"
  echo "Enabled launchd agent com.todo.reminder"
}

install_cron() {
  if ! command -v crontab >/dev/null 2>&1; then
    echo "crontab not found. Install cron or choose launchd." >&2
    exit 1
  fi

  local cron_line="* * * * * $INSTALL_DIR/todo notify"
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
    echo "1) launchd (recommended)"
    echo "2) cron"
    printf "> "
    read -r choice
  fi

  case "$choice" in
    2|cron|CRON)
      install_cron
      ;;
    *)
      install_launchd
      ;;
  esac
}

choose_scheduler "${TODO_SCHEDULER:-}"

echo "Installed todo to $INSTALL_DIR/todo"
if ! echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
  echo "Note: $INSTALL_DIR is not in your PATH. Add it to your shell profile."
fi
