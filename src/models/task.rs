//! Task model and related types

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Archived,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in-progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "in-progress" | "inprogress" | "in_progress" => Ok(TaskStatus::InProgress),
            "completed" | "done" => Ok(TaskStatus::Completed),
            "archived" => Ok(TaskStatus::Archived),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Priority::Low),
            "medium" | "med" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            "critical" | "crit" => Ok(Priority::Critical),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}

/// Task kind/type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    #[default]
    Task,
    Todo,
    Idea,
}

impl fmt::Display for TaskKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskKind::Task => write!(f, "task"),
            TaskKind::Todo => write!(f, "todo"),
            TaskKind::Idea => write!(f, "idea"),
        }
    }
}

impl std::str::FromStr for TaskKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(TaskKind::Task),
            "todo" => Ok(TaskKind::Todo),
            "idea" => Ok(TaskKind::Idea),
            _ => Err(format!("Unknown kind: {}", s)),
        }
    }
}

/// A task with all its metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub kind: TaskKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<NaiveDate>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_commit: Option<String>,
    /// The markdown body (not part of frontmatter)
    #[serde(skip)]
    pub description: String,
}

impl Task {
    /// Create a new task with the given kind and title
    pub fn new(id: u64, kind: TaskKind, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Task {
            id,
            title: title.into(),
            status: TaskStatus::default(),
            priority: Priority::default(),
            kind,
            tags: Vec::new(),
            due: None,
            created: now,
            updated: now,
            closed_commit: None,
            description: String::new(),
        }
    }

    /// Generate a slug from the title
    pub fn slug(&self) -> String {
        slug::slugify(&self.title)
    }

    /// Generate the filename for this task
    pub fn filename(&self) -> String {
        format!("{}-{:03}.md", self.slug(), self.id)
    }

    /// Check if the task is open (not completed or archived)
    pub fn is_open(&self) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::InProgress)
    }

    /// Mark the task as completed with the given commit hash
    pub fn complete(&mut self, commit: Option<String>) {
        self.status = TaskStatus::Completed;
        self.closed_commit = commit;
        self.updated = Utc::now();
    }

    /// Update the task's updated timestamp
    pub fn touch(&mut self) {
        self.updated = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::InProgress.to_string(), "in-progress");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Archived.to_string(), "archived");
    }

    #[test]
    fn test_task_status_parse() {
        assert_eq!(
            "pending".parse::<TaskStatus>().unwrap(),
            TaskStatus::Pending
        );
        assert_eq!(
            "in-progress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            "inprogress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            "completed".parse::<TaskStatus>().unwrap(),
            TaskStatus::Completed
        );
        assert_eq!("done".parse::<TaskStatus>().unwrap(), TaskStatus::Completed);
        assert!("invalid".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Low.to_string(), "low");
        assert_eq!(Priority::Medium.to_string(), "medium");
        assert_eq!(Priority::High.to_string(), "high");
        assert_eq!(Priority::Critical.to_string(), "critical");
    }

    #[test]
    fn test_priority_parse() {
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
        assert_eq!("medium".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("med".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Critical);
        assert!("invalid".parse::<Priority>().is_err());
    }

    #[test]
    fn test_task_kind_display() {
        assert_eq!(TaskKind::Task.to_string(), "task");
        assert_eq!(TaskKind::Todo.to_string(), "todo");
        assert_eq!(TaskKind::Idea.to_string(), "idea");
    }

    #[test]
    fn test_task_kind_parse() {
        assert_eq!("task".parse::<TaskKind>().unwrap(), TaskKind::Task);
        assert_eq!("todo".parse::<TaskKind>().unwrap(), TaskKind::Todo);
        assert_eq!("idea".parse::<TaskKind>().unwrap(), TaskKind::Idea);
        assert!("invalid".parse::<TaskKind>().is_err());
    }

    #[test]
    fn test_task_new() {
        let task = Task::new(1, TaskKind::Task, "Fix authentication bug");
        assert_eq!(task.id, 1);
        assert_eq!(task.title, "Fix authentication bug");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.kind, TaskKind::Task);
        assert!(task.tags.is_empty());
        assert!(task.due.is_none());
        assert!(task.closed_commit.is_none());
    }

    #[test]
    fn test_task_slug() {
        let task = Task::new(1, TaskKind::Task, "Fix Authentication Bug");
        assert_eq!(task.slug(), "fix-authentication-bug");
    }

    #[test]
    fn test_task_filename() {
        let task = Task::new(1, TaskKind::Task, "Fix auth bug");
        assert_eq!(task.filename(), "fix-auth-bug-001.md");

        let task2 = Task::new(123, TaskKind::Task, "Test");
        assert_eq!(task2.filename(), "test-123.md");
    }

    #[test]
    fn test_task_is_open() {
        let mut task = Task::new(1, TaskKind::Task, "Test");
        assert!(task.is_open());

        task.status = TaskStatus::InProgress;
        assert!(task.is_open());

        task.status = TaskStatus::Completed;
        assert!(!task.is_open());

        task.status = TaskStatus::Archived;
        assert!(!task.is_open());
    }

    #[test]
    fn test_task_complete() {
        let mut task = Task::new(1, TaskKind::Task, "Test");
        task.complete(Some("abc123".to_string()));

        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.closed_commit, Some("abc123".to_string()));
    }
}
