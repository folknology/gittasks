# gittask

Git-versioned task management using Markdown files with YAML frontmatter.

## Building

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run linter
cargo clippy

# Format code
cargo fmt
```

## Installation

```bash
# Install from local source
cargo install --path .

# Or run directly
./target/release/gittask
```

## CLI Usage

### Initialize

Create a `.tasks` directory in the current git repository:

```bash
gittask init
```

### Adding Tasks

```bash
# Add a task
gittask add task "Implement login feature"

# Add a todo
gittask add todo "Review pull request"

# Add an idea
gittask add idea "Consider using Redis for caching"

# Add with options
gittask add task "Fix authentication bug" \
  --description "Users are being logged out unexpectedly" \
  --priority high \
  --due 2026-02-20 \
  --tags bug,auth
```

### Listing Tasks

```bash
# List all tasks
gittask list

# Filter by kind
gittask list --kind todo

# Filter by status
gittask list --status pending

# Filter by priority
gittask list --priority high

# Filter by tags
gittask list --tags bug,urgent

# Include archived tasks
gittask list --include-archived
```

### Viewing Tasks

```bash
# Show task details
gittask show 1
```

### Updating Tasks

```bash
# Update title
gittask update 1 --title "New title"

# Update priority
gittask update 1 --priority critical

# Update due date
gittask update 1 --due 2026-03-01

# Update tags (replaces existing)
gittask update 1 --tags feature,frontend

# Update description
gittask update 1 --description "Updated description"
```

### Changing Status

```bash
# Mark as in-progress
gittask status 1 in-progress

# Mark as completed (captures git commit)
gittask status 1 completed

# Archive a task
gittask status 1 archived
```

### Completing Tasks

```bash
# Complete one task
gittask complete 1

# Complete multiple tasks
gittask complete 1 2 3
```

### Deleting Tasks

```bash
# Delete with confirmation
gittask delete 1

# Delete without confirmation
gittask delete 1 --force
```

### Statistics

```bash
gittask stats
```

## Global Mode

Use `--global` or `-g` to work with tasks in `~/.tasks` instead of the current project:

```bash
gittask -g init
gittask -g add task "Personal task"
gittask -g list
```

## Project Registry (Multi-Project Aggregation)

Register projects to aggregate tasks across multiple repositories.

### Register Projects

```bash
# Register current project
gittask link

# Register a specific project
gittask link /path/to/project
```

### Unregister Projects

```bash
# Unregister current project
gittask unlink

# Unregister a specific project
gittask unlink /path/to/project
```

### View Registered Projects

```bash
gittask projects
```

Output:
```
+----------+---------------------+--------+------+-------+
| Project  | Path                | Status | Open | Total |
+----------+---------------------+--------+------+-------+
| gittask  | /Users/me/gittask   | ok     |    3 |     5 |
| webapp   | /Users/me/webapp    | ok     |    7 |    12 |
+----------+---------------------+--------+------+-------+
```

### Aggregated Task View

When projects are registered, `gittask -g list` shows tasks from all projects:

```bash
gittask -g list
```

Output:
```
+-----------+---------+------+------------------+-----------+----------+-----+
|        ID | Project | Kind | Title            | Status    | Priority | Due |
+-----------+---------+------+------------------+-----------+----------+-----+
| gittask:1 | gittask | task | Add tests        | pending   | high     |     |
| gittask:2 | gittask | task | Write docs       | pending   | medium   |     |
|  webapp:1 | webapp  | task | Fix login bug    | pending   | critical |     |
|  webapp:3 | webapp  | todo | Update deps      | pending   | low      |     |
+-----------+---------+------+------------------+-----------+----------+-----+
```

### Qualified IDs

Use `project:id` format to work with tasks across projects:

```bash
# View task from specific project
gittask show webapp:1

# Complete task from specific project
gittask complete gittask:2

# Update task from specific project
gittask update webapp:3 --priority high

# Change status
gittask status gittask:1 in-progress
```

## MCP Server

gittask includes an MCP (Model Context Protocol) server for integration with AI assistants like Claude.

### Running the MCP Server

```bash
# Run for current project
gittask-mcp

# Run for global tasks
gittask-mcp --global
```

### MCP Configuration

Add to your Claude Code MCP settings:

```json
{
  "mcpServers": {
    "gittask": {
      "command": "gittask-mcp",
      "args": []
    }
  }
}
```

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `add_task` | Create a new task/todo/idea |
| `list_tasks` | List tasks with filters (supports `aggregate: true`) |
| `get_task` | Get task details by ID |
| `complete_task` | Mark tasks as completed |
| `update_task` | Update task properties |
| `delete_task` | Delete a task |
| `set_task_status` | Change task status |
| `get_stats` | Get task statistics |
| `link_project` | Register a project |
| `unlink_project` | Unregister a project |
| `list_projects` | List registered projects |

### MCP Aggregation

Use the `aggregate` parameter with `list_tasks` to get tasks from all registered projects:

```json
{
  "name": "list_tasks",
  "arguments": {
    "aggregate": true,
    "status": "pending"
  }
}
```

### MCP Qualified IDs

MCP tools accept both numeric IDs and qualified IDs:

```json
{
  "name": "get_task",
  "arguments": {
    "id": "webapp:1"
  }
}
```

## Task File Format

Tasks are stored as Markdown files with YAML frontmatter in `.tasks/`:

```markdown
---
id: 1
title: Implement login feature
status: pending
priority: high
kind: task
tags:
  - auth
  - feature
due: 2026-02-20
created: 2026-02-13T10:30:00Z
updated: 2026-02-13T10:30:00Z
---

Detailed description goes here.

Can include multiple paragraphs and markdown formatting.
```

### Status Values

- `pending` - Not started
- `in-progress` - Currently being worked on
- `completed` - Done (captures git commit hash)
- `archived` - No longer relevant

### Priority Values

- `low`
- `medium` (default)
- `high`
- `critical`

### Kind Values

- `task` - A work item to complete
- `todo` - A quick action item
- `idea` - Something to consider later

## Registry File

The project registry is stored at `~/.tasks/.projects`:

```
/Users/me/gittask
/Users/me/webapp
/Users/me/api-server
```

Each line is an absolute path to a registered project.
