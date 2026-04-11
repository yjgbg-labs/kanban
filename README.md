# Kanban

A full-stack Kanban application with REST API, command-line client, and web UI.

| Directory | Description | Tech |
|-----------|-------------|------|
| `server/` | Web server with REST API | Rust · Axum · SQLx · SQLite |
| `cli/` | Command-line client | Rust · Clap · Reqwest |
| `web-ui/` | Browser-based kanban board | Remix v2 · React 19 · Vite |

## Getting started

### Server

```bash
cd server && cargo run
```

The server listens on `http://localhost:3001`.

### CLI

```bash
cd cli && cargo build --release
# Binary at cli/target/release/kanban
```

Usage:

```bash
# Set server URL (default: http://localhost:3001)
export SERVER_URL=http://localhost:3001

kanban kanban list
kanban kanban create -t "My Board"
kanban column create <kanban-id> -t "To Do"
kanban card create <column-id> -t "First Task" -d "Description"
```

Run `kanban --help` for all available commands.

### Web UI

```bash
cd web-ui && npm install && npm run dev
```
