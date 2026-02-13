//! MCP server implementation using JSON-RPC over stdio
//!
//! This is a manual implementation of the MCP protocol for maximum control
//! and simpler debugging.

use crate::git::GitOperations;
use crate::models::{Task, TaskKind, TaskStatus};
use crate::storage::{FileStore, TaskFilter, TaskLocation};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// JSON-RPC request
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// JSON-RPC response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

/// Task output for MCP responses
#[derive(Serialize)]
struct TaskOutput {
    id: u64,
    title: String,
    kind: String,
    status: String,
    priority: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    due: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    closed_commit: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
}

impl From<&Task> for TaskOutput {
    fn from(task: &Task) -> Self {
        TaskOutput {
            id: task.id,
            title: task.title.clone(),
            kind: task.kind.to_string(),
            status: task.status.to_string(),
            priority: task.priority.to_string(),
            tags: task.tags.clone(),
            due: task.due.map(|d| d.to_string()),
            closed_commit: task.closed_commit.clone(),
            description: task.description.clone(),
        }
    }
}

/// MCP Server state
pub struct McpServer {
    global: bool,
}

impl McpServer {
    pub fn new(global: bool) -> Self {
        Self { global }
    }

    fn get_store(&self) -> Result<FileStore, String> {
        let location = if self.global {
            TaskLocation::global().map_err(|e| e.to_string())?
        } else {
            TaskLocation::find_project().map_err(|e| e.to_string())?
        };
        Ok(FileStore::new(location))
    }

    /// Handle a JSON-RPC request and return a response
    fn handle_request(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone().unwrap_or(Value::Null);

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id),
            "initialized" => JsonRpcResponse::success(id, json!({})),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, request.params.as_ref()),
            _ => {
                JsonRpcResponse::error(id, -32601, format!("Method not found: {}", request.method))
            }
        }
    }

    fn handle_initialize(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "gittask",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let tools = json!({
            "tools": [
                {
                    "name": "add_task",
                    "description": "Create a new task, todo, or idea",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "description": "Type: task, todo, or idea"},
                            "title": {"type": "string", "description": "Task title"},
                            "description": {"type": "string", "description": "Optional description"},
                            "priority": {"type": "string", "description": "Priority: low, medium, high, critical"},
                            "due": {"type": "string", "description": "Due date YYYY-MM-DD"},
                            "tags": {"type": "array", "items": {"type": "string"}}
                        },
                        "required": ["kind", "title"]
                    }
                },
                {
                    "name": "list_tasks",
                    "description": "List tasks with optional filters",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string"},
                            "status": {"type": "string"},
                            "priority": {"type": "string"},
                            "tags": {"type": "array", "items": {"type": "string"}},
                            "include_archived": {"type": "boolean"}
                        }
                    }
                },
                {
                    "name": "get_task",
                    "description": "Get task details by ID",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer", "description": "Task ID"}
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "complete_task",
                    "description": "Mark task(s) as completed, capturing git commit",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "ids": {"type": "array", "items": {"type": "integer"}}
                        },
                        "required": ["ids"]
                    }
                },
                {
                    "name": "update_task",
                    "description": "Update task properties",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "title": {"type": "string"},
                            "description": {"type": "string"},
                            "priority": {"type": "string"},
                            "due": {"type": "string"},
                            "tags": {"type": "array", "items": {"type": "string"}}
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "delete_task",
                    "description": "Delete a task",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"}
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "set_task_status",
                    "description": "Change task status",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "status": {"type": "string", "description": "pending, in-progress, completed, archived"}
                        },
                        "required": ["id", "status"]
                    }
                },
                {
                    "name": "get_stats",
                    "description": "Get task statistics",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        });

        JsonRpcResponse::success(id, tools)
    }

    fn handle_tools_call(&self, id: Value, params: Option<&Value>) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => return JsonRpcResponse::error(id, -32602, "Missing params".to_string()),
        };

        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match name {
            "add_task" => self.tool_add_task(&args),
            "list_tasks" => self.tool_list_tasks(&args),
            "get_task" => self.tool_get_task(&args),
            "complete_task" => self.tool_complete_task(&args),
            "update_task" => self.tool_update_task(&args),
            "delete_task" => self.tool_delete_task(&args),
            "set_task_status" => self.tool_set_task_status(&args),
            "get_stats" => self.tool_get_stats(&args),
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match result {
            Ok(content) => JsonRpcResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&content).unwrap_or_default()
                    }]
                }),
            ),
            Err(e) => JsonRpcResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error: {}", e)
                    }],
                    "isError": true
                }),
            ),
        }
    }

    fn tool_add_task(&self, args: &Value) -> Result<Value, String> {
        let kind: TaskKind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'kind'")?
            .parse()?;

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'title'")?;

        let mut task = Task::new(0, kind, title);

        if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
            task.description = desc.to_string();
        }

        if let Some(p) = args.get("priority").and_then(|v| v.as_str()) {
            task.priority = p.parse()?;
        }

        if let Some(due) = args.get("due").and_then(|v| v.as_str()) {
            task.due = Some(
                NaiveDate::parse_from_str(due, "%Y-%m-%d")
                    .map_err(|e| format!("Invalid date: {}", e))?,
            );
        }

        if let Some(tags) = args.get("tags").and_then(|v| v.as_array()) {
            task.tags = tags
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        let store = self.get_store()?;
        store
            .location()
            .ensure_exists()
            .map_err(|e| e.to_string())?;

        let created = store.create(task).map_err(|e| e.to_string())?;
        Ok(json!(TaskOutput::from(&created)))
    }

    fn tool_list_tasks(&self, args: &Value) -> Result<Value, String> {
        let filter = TaskFilter {
            kind: args
                .get("kind")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            status: args
                .get("status")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            priority: args
                .get("priority")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            tags: args
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            include_archived: args
                .get("include_archived")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };

        let store = self.get_store()?;
        let tasks = store.list(&filter).map_err(|e| e.to_string())?;

        let output: Vec<TaskOutput> = tasks.iter().map(TaskOutput::from).collect();
        Ok(json!(output))
    }

    fn tool_get_task(&self, args: &Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'id'")?;

        let store = self.get_store()?;
        let task = store.read(id).map_err(|e| e.to_string())?;

        Ok(json!(TaskOutput::from(&task)))
    }

    fn tool_complete_task(&self, args: &Value) -> Result<Value, String> {
        let ids: Vec<u64> = args
            .get("ids")
            .and_then(|v| v.as_array())
            .ok_or("Missing 'ids'")?
            .iter()
            .filter_map(|v| v.as_u64())
            .collect();

        let store = self.get_store()?;
        let commit = GitOperations::head_commit_optional(&store.location().root);

        let mut completed = Vec::new();
        for id in ids {
            let mut task = store.read(id).map_err(|e| e.to_string())?;
            task.complete(commit.clone());
            store.update(&task).map_err(|e| e.to_string())?;
            completed.push(TaskOutput::from(&task));
        }

        Ok(json!(completed))
    }

    fn tool_update_task(&self, args: &Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'id'")?;

        let store = self.get_store()?;
        let mut task = store.read(id).map_err(|e| e.to_string())?;

        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            task.title = title.to_string();
        }

        if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
            task.description = desc.to_string();
        }

        if let Some(p) = args.get("priority").and_then(|v| v.as_str()) {
            task.priority = p.parse()?;
        }

        if let Some(due) = args.get("due").and_then(|v| v.as_str()) {
            task.due = Some(
                NaiveDate::parse_from_str(due, "%Y-%m-%d")
                    .map_err(|e| format!("Invalid date: {}", e))?,
            );
        }

        if let Some(tags) = args.get("tags").and_then(|v| v.as_array()) {
            task.tags = tags
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        task.touch();
        store.update(&task).map_err(|e| e.to_string())?;

        Ok(json!(TaskOutput::from(&task)))
    }

    fn tool_delete_task(&self, args: &Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'id'")?;

        let store = self.get_store()?;
        store.delete(id).map_err(|e| e.to_string())?;

        Ok(json!({"deleted": id}))
    }

    fn tool_set_task_status(&self, args: &Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or("Missing 'id'")?;

        let status: TaskStatus = args
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'status'")?
            .parse()?;

        let store = self.get_store()?;
        let mut task = store.read(id).map_err(|e| e.to_string())?;

        // If completing, capture git commit
        if status == TaskStatus::Completed && task.status != TaskStatus::Completed {
            let commit = GitOperations::head_commit_optional(&store.location().root);
            task.closed_commit = commit;
        }

        task.status = status;
        task.touch();
        store.update(&task).map_err(|e| e.to_string())?;

        Ok(json!(TaskOutput::from(&task)))
    }

    fn tool_get_stats(&self, _args: &Value) -> Result<Value, String> {
        let store = self.get_store()?;
        let stats = store.stats().map_err(|e| e.to_string())?;

        Ok(json!({
            "total": stats.total,
            "pending": stats.pending,
            "in_progress": stats.in_progress,
            "completed": stats.completed,
            "archived": stats.archived,
            "overdue": stats.overdue,
            "by_kind": {
                "tasks": stats.tasks,
                "todos": stats.todos,
                "ideas": stats.ideas
            }
        }))
    }
}

/// Run the MCP server (async stdio)
pub async fn run_mcp_server(global: bool) -> anyhow::Result<()> {
    let server = McpServer::new(global);

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = tokio::io::BufReader::new(stdin);

    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // EOF
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(request) => {
                // Handle notifications (no id) silently
                if request.id.is_none() && request.method == "notifications/initialized" {
                    continue;
                }

                let response = server.handle_request(&request);

                // Only send response if there was an id (not a notification)
                if request.id.is_some() {
                    let response_json = serde_json::to_string(&response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
            Err(e) => {
                let response =
                    JsonRpcResponse::error(Value::Null, -32700, format!("Parse error: {}", e));
                let response_json = serde_json::to_string(&response)?;
                stdout.write_all(response_json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }
    }

    Ok(())
}
