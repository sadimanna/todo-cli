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

mkdir -p "$HOME/.config/systemd/user"
cp "$ROOT_DIR/systemd/todo-reminder.service" "$HOME/.config/systemd/user/"
cp "$ROOT_DIR/systemd/todo-reminder.timer" "$HOME/.config/systemd/user/"

systemctl --user daemon-reload
systemctl --user enable todo-reminder.timer
systemctl --user start todo-reminder.timer

echo "Installed todo to $HOME/.local/bin/todo"
echo "Enabled systemd timer todo-reminder.timer"
