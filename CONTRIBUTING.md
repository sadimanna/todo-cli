# Contributing

Thanks for contributing to todo.

## Setup
1. Install Rust (rustup recommended).
2. Install system dependencies:
   - Debian/Ubuntu: `sudo apt-get install -y libsqlite3-dev libnotify-dev`
3. Build:
   - `cargo build`

## Development
- Run tests: `cargo test`
- Format: `cargo fmt`
- Lint (optional): `cargo clippy`

## Code Style
- Keep functions small and focused.
- Prefer explicit error handling.
- Avoid heavy background work in the TUI loop.

## Pull Requests
- Include a clear description of changes.
- Add or update tests when behavior changes.
- Ensure `cargo test` passes.
