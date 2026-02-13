//! File-based storage for tasks

use crate::models::{
    FrontmatterError, Priority, Task, TaskKind, TaskStatus, parse_task, serialize_task,
};
use crate::storage::id_generator::IdGenerator;
use crate::storage::location::TaskLocation;
use crate::storage::registry::ProjectRegistry;
use std::path::PathBuf;
use thiserror::Error;

/// Errors related to file storage operations
#[derive(Debug, Error)]
pub enum FileStoreError {
    #[error("Task not found: {0}")]
    TaskNotFound(u64),
    #[error("Frontmatter error: {0}")]
    Frontmatter(#[from] FrontmatterError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Task directory does not exist. Run 'gittask init' first.")]
    DirectoryNotInitialized,
}

/// Filter criteria for listing tasks
#[derive(Debug, Default, Clone)]
pub struct TaskFilter {
    pub kind: Option<TaskKind>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    pub include_archived: bool,
}

impl TaskFilter {
    /// Check if a task matches the filter criteria
    pub fn matches(&self, task: &Task) -> bool {
        // Filter by kind
        if let Some(kind) = self.kind
            && task.kind != kind
        {
            return false;
        }

        // Filter by status
        if let Some(status) = self.status
            && task.status != status
        {
            return false;
        }

        // Filter by priority
        if let Some(priority) = self.priority
            && task.priority != priority
        {
            return false;
        }

        // Filter by tags (all specified tags must be present)
        if !self.tags.is_empty() {
            for tag in &self.tags {
                if !task.tags.contains(tag) {
                    return false;
                }
            }
        }

        // Exclude archived unless explicitly included
        if !self.include_archived && task.status == TaskStatus::Archived {
            return false;
        }

        true
    }
}

/// File-based task storage
pub struct FileStore {
    location: TaskLocation,
}

impl FileStore {
    /// Create a new file store for the given location
    pub fn new(location: TaskLocation) -> Self {
        FileStore { location }
    }

    /// Get the task location
    pub fn location(&self) -> &TaskLocation {
        &self.location
    }

    /// Create a new task and return it with its assigned ID
    pub fn create(&self, mut task: Task) -> Result<Task, FileStoreError> {
        if !self.location.exists() {
            return Err(FileStoreError::DirectoryNotInitialized);
        }

        // Generate the next ID
        let id = IdGenerator::next_id(&self.location.tasks_dir)
            .map_err(|e| FileStoreError::Io(std::io::Error::other(e.to_string())))?;
        task.id = id;

        // Write the task file
        let path = self.task_path(&task);
        let content = serialize_task(&task)?;
        std::fs::write(&path, content)?;

        Ok(task)
    }

    /// Read a task by ID
    pub fn read(&self, id: u64) -> Result<Task, FileStoreError> {
        let path = self.find_task_file(id)?;
        let content = std::fs::read_to_string(&path)?;
        let task = parse_task(&content)?;
        Ok(task)
    }

    /// Update an existing task
    pub fn update(&self, task: &Task) -> Result<(), FileStoreError> {
        // Find and delete the old file (filename might have changed if title changed)
        let old_path = self.find_task_file(task.id)?;
        let new_path = self.task_path(task);

        if old_path != new_path {
            std::fs::remove_file(&old_path)?;
        }

        let content = serialize_task(task)?;
        std::fs::write(&new_path, content)?;

        Ok(())
    }

    /// Delete a task by ID
    pub fn delete(&self, id: u64) -> Result<(), FileStoreError> {
        let path = self.find_task_file(id)?;
        std::fs::remove_file(&path)?;
        Ok(())
    }

    /// List all tasks, optionally filtered
    pub fn list(&self, filter: &TaskFilter) -> Result<Vec<Task>, FileStoreError> {
        if !self.location.exists() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();

        for entry in std::fs::read_dir(&self.location.tasks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "md") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => match parse_task(&content) {
                        Ok(task) => {
                            if filter.matches(&task) {
                                tasks.push(task);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse task file {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to read task file {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by ID
        tasks.sort_by_key(|t| t.id);

        Ok(tasks)
    }

    /// Get statistics about tasks
    pub fn stats(&self) -> Result<TaskStats, FileStoreError> {
        let all_tasks = self.list(&TaskFilter {
            include_archived: true,
            ..Default::default()
        })?;

        let mut stats = TaskStats {
            total: all_tasks.len(),
            ..Default::default()
        };

        for task in &all_tasks {
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::InProgress => stats.in_progress += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Archived => stats.archived += 1,
            }

            match task.kind {
                TaskKind::Task => stats.tasks += 1,
                TaskKind::Todo => stats.todos += 1,
                TaskKind::Idea => stats.ideas += 1,
            }

            // Check for overdue
            if task.is_open()
                && let Some(due) = task.due
                && due < chrono::Utc::now().date_naive()
            {
                stats.overdue += 1;
            }
        }

        Ok(stats)
    }

    /// Get the path for a task file
    fn task_path(&self, task: &Task) -> PathBuf {
        self.location.tasks_dir.join(task.filename())
    }

    /// Find the file for a task by ID
    fn find_task_file(&self, id: u64) -> Result<PathBuf, FileStoreError> {
        if !self.location.exists() {
            return Err(FileStoreError::DirectoryNotInitialized);
        }

        for entry in std::fs::read_dir(&self.location.tasks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "md")
                && let Some(file_id) = IdGenerator::extract_id_from_filename(&path)
                && file_id == id
            {
                return Ok(path);
            }
        }

        Err(FileStoreError::TaskNotFound(id))
    }
}

/// Task statistics
#[derive(Debug, Default, Clone)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub archived: usize,
    pub overdue: usize,
    pub tasks: usize,
    pub todos: usize,
    pub ideas: usize,
}

/// A task with its project context for aggregated views
#[derive(Debug, Clone)]
pub struct AggregatedTask {
    /// The task itself
    pub task: Task,
    /// Project name (directory name)
    pub project: String,
    /// Project root path
    pub project_path: PathBuf,
}

impl AggregatedTask {
    /// Get the qualified ID (project:id format)
    pub fn qualified_id(&self) -> String {
        format!("{}:{}", self.project, self.task.id)
    }
}

/// List tasks aggregated from all registered projects
pub fn list_aggregated(
    registry: &ProjectRegistry,
    filter: &TaskFilter,
) -> Result<Vec<AggregatedTask>, FileStoreError> {
    let mut results = Vec::new();

    for project_path in registry.projects() {
        let project_name = project_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.to_string_lossy().to_string());

        // Skip projects that don't exist
        if !project_path.exists() {
            log::warn!("Project path does not exist: {}", project_path.display());
            continue;
        }

        // Try to get tasks from this project
        match TaskLocation::find_project_from(project_path) {
            Ok(location) => {
                let store = FileStore::new(location.clone());
                match store.list(filter) {
                    Ok(tasks) => {
                        for task in tasks {
                            results.push(AggregatedTask {
                                task,
                                project: project_name.clone(),
                                project_path: location.root.clone(),
                            });
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to list tasks from {}: {}",
                            project_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to find project at {}: {}",
                    project_path.display(),
                    e
                );
            }
        }
    }

    // Sort by project name, then by task ID
    results.sort_by(|a, b| {
        a.project
            .cmp(&b.project)
            .then_with(|| a.task.id.cmp(&b.task.id))
    });

    Ok(results)
}

/// Resolve a qualified ID (e.g., "gittask:1" or just "1")
/// Returns (project_path, task_id) if found
pub fn resolve_qualified_id(
    id_str: &str,
    registry: &ProjectRegistry,
    default_location: Option<&TaskLocation>,
) -> Result<(TaskLocation, u64), String> {
    if let Some((project_name, id_part)) = id_str.split_once(':') {
        // Qualified ID: "project:id"
        let task_id: u64 = id_part
            .parse()
            .map_err(|_| format!("Invalid task ID: {}", id_part))?;

        let project_path = registry
            .find_project(project_name)
            .ok_or_else(|| format!("Project not found: {}", project_name))?;

        let location = TaskLocation::find_project_from(&project_path)
            .map_err(|e| format!("Failed to find project: {}", e))?;

        Ok((location, task_id))
    } else {
        // Local ID: just a number
        let task_id: u64 = id_str
            .parse()
            .map_err(|_| format!("Invalid task ID: {}", id_str))?;

        let location = default_location
            .cloned()
            .ok_or_else(|| "No default location available".to_string())?;

        Ok((location, task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_store() -> (TempDir, FileStore) {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        let location = TaskLocation::find_project_from(temp.path()).unwrap();
        location.ensure_exists().unwrap();
        let store = FileStore::new(location);
        (temp, store)
    }

    #[test]
    fn test_create_task() {
        let (_temp, store) = setup_test_store();

        let task = Task::new(0, TaskKind::Task, "Test task");
        let created = store.create(task).unwrap();

        assert_eq!(created.id, 1);
        assert_eq!(created.title, "Test task");

        // Verify file exists
        let files: Vec<_> = std::fs::read_dir(&store.location.tasks_dir)
            .unwrap()
            .collect();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_read_task() {
        let (_temp, store) = setup_test_store();

        let task = Task::new(0, TaskKind::Task, "Test task");
        let created = store.create(task).unwrap();

        let read = store.read(created.id).unwrap();
        assert_eq!(read.id, created.id);
        assert_eq!(read.title, "Test task");
    }

    #[test]
    fn test_update_task() {
        let (_temp, store) = setup_test_store();

        let task = Task::new(0, TaskKind::Task, "Original title");
        let mut created = store.create(task).unwrap();

        created.title = "Updated title".to_string();
        created.priority = Priority::High;
        store.update(&created).unwrap();

        let read = store.read(created.id).unwrap();
        assert_eq!(read.title, "Updated title");
        assert_eq!(read.priority, Priority::High);
    }

    #[test]
    fn test_delete_task() {
        let (_temp, store) = setup_test_store();

        let task = Task::new(0, TaskKind::Task, "Test task");
        let created = store.create(task).unwrap();

        store.delete(created.id).unwrap();

        assert!(store.read(created.id).is_err());
    }

    #[test]
    fn test_list_tasks() {
        let (_temp, store) = setup_test_store();

        store
            .create(Task::new(0, TaskKind::Task, "Task 1"))
            .unwrap();
        store
            .create(Task::new(0, TaskKind::Todo, "Todo 1"))
            .unwrap();
        store
            .create(Task::new(0, TaskKind::Idea, "Idea 1"))
            .unwrap();

        let all = store.list(&TaskFilter::default()).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_filter_by_kind() {
        let (_temp, store) = setup_test_store();

        store
            .create(Task::new(0, TaskKind::Task, "Task 1"))
            .unwrap();
        store
            .create(Task::new(0, TaskKind::Todo, "Todo 1"))
            .unwrap();

        let filter = TaskFilter {
            kind: Some(TaskKind::Task),
            ..Default::default()
        };
        let tasks = store.list(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].kind, TaskKind::Task);
    }

    #[test]
    fn test_filter_by_status() {
        let (_temp, store) = setup_test_store();

        let mut task1 = store
            .create(Task::new(0, TaskKind::Task, "Task 1"))
            .unwrap();
        store
            .create(Task::new(0, TaskKind::Task, "Task 2"))
            .unwrap();

        task1.status = TaskStatus::Completed;
        store.update(&task1).unwrap();

        let filter = TaskFilter {
            status: Some(TaskStatus::Pending),
            ..Default::default()
        };
        let tasks = store.list(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_filter_excludes_archived() {
        let (_temp, store) = setup_test_store();

        let mut task = store
            .create(Task::new(0, TaskKind::Task, "Task 1"))
            .unwrap();
        task.status = TaskStatus::Archived;
        store.update(&task).unwrap();

        let filter = TaskFilter::default();
        let tasks = store.list(&filter).unwrap();
        assert_eq!(tasks.len(), 0);

        let filter = TaskFilter {
            include_archived: true,
            ..Default::default()
        };
        let tasks = store.list(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_filter_by_tags() {
        let (_temp, store) = setup_test_store();

        let mut task1 = Task::new(0, TaskKind::Task, "Task 1");
        task1.tags = vec!["bug".to_string(), "urgent".to_string()];
        store.create(task1).unwrap();

        let mut task2 = Task::new(0, TaskKind::Task, "Task 2");
        task2.tags = vec!["feature".to_string()];
        store.create(task2).unwrap();

        let filter = TaskFilter {
            tags: vec!["bug".to_string()],
            ..Default::default()
        };
        let tasks = store.list(&filter).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].tags.contains(&"bug".to_string()));
    }

    #[test]
    fn test_stats() {
        let (_temp, store) = setup_test_store();

        store
            .create(Task::new(0, TaskKind::Task, "Task 1"))
            .unwrap();
        store
            .create(Task::new(0, TaskKind::Todo, "Todo 1"))
            .unwrap();

        let mut task = store
            .create(Task::new(0, TaskKind::Idea, "Idea 1"))
            .unwrap();
        task.status = TaskStatus::Completed;
        store.update(&task).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.tasks, 1);
        assert_eq!(stats.todos, 1);
        assert_eq!(stats.ideas, 1);
    }

    #[test]
    fn test_sequential_ids() {
        let (_temp, store) = setup_test_store();

        let task1 = store
            .create(Task::new(0, TaskKind::Task, "Task 1"))
            .unwrap();
        let task2 = store
            .create(Task::new(0, TaskKind::Task, "Task 2"))
            .unwrap();
        let task3 = store
            .create(Task::new(0, TaskKind::Task, "Task 3"))
            .unwrap();

        assert_eq!(task1.id, 1);
        assert_eq!(task2.id, 2);
        assert_eq!(task3.id, 3);
    }
}
