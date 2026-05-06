# Tick

Tick is a Tauri + React + TypeScript desktop app for managing macOS user `launchd` jobs.

## Features

- Create, edit, delete, enable, and disable Tick-managed LaunchAgents.
- Configure calendar schedules with month, day, hour, minute, and second fields.
- Configure interval schedules that run every N seconds.
- Run inline shell scripts, executable script paths, or interpreter-based commands such as Node.js.
- Edit inline scripts with syntax highlighting.
- Inspect stdout/stderr logs, clear logs, and auto-refresh logs.
- Preview the generated plist for each job.

## Launchd Behavior

Tick only manages user LaunchAgents in:

```text
~/Library/LaunchAgents
```

Tick labels use this prefix:

```text
com.gavin.tick.
```

Logs and managed inline scripts live in the app data directory under `tick`.

`launchd` calendar schedules support month, day, hour, and minute, but not seconds. Tick handles calendar seconds by generating a wrapper script that sleeps for the configured number of seconds before executing the command. Interval schedules use native `StartInterval`.

LaunchAgents do not load your interactive shell profile. Use absolute paths for interpreters, scripts, and working directories, for example `/opt/homebrew/bin/node`.

## Development

Install dependencies:

```bash
npm install
```

Run the web UI:

```bash
npm run dev
```

Run the desktop app:

```bash
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Check and test the Rust backend:

```bash
cd src-tauri
cargo check
cargo test
```
