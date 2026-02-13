//! YAML frontmatter parsing and serialization

use crate::models::task::Task;
use thiserror::Error;

/// Frontmatter delimiter
const FRONTMATTER_DELIMITER: &str = "---";

/// Errors that can occur during frontmatter operations
#[derive(Debug, Error)]
pub enum FrontmatterError {
    #[error("Missing frontmatter delimiters")]
    MissingDelimiters,
    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("Invalid frontmatter format")]
    InvalidFormat,
}

/// Parse a markdown file with YAML frontmatter into a Task
pub fn parse_task(content: &str) -> Result<Task, FrontmatterError> {
    let (frontmatter, body) = split_frontmatter(content)?;
    let mut task: Task = serde_yaml::from_str(&frontmatter)?;
    task.description = body.trim().to_string();
    Ok(task)
}

/// Serialize a Task to a markdown file with YAML frontmatter
pub fn serialize_task(task: &Task) -> Result<String, FrontmatterError> {
    let frontmatter = serde_yaml::to_string(&task)?;
    let mut result = String::new();
    result.push_str(FRONTMATTER_DELIMITER);
    result.push('\n');
    result.push_str(&frontmatter);
    result.push_str(FRONTMATTER_DELIMITER);
    result.push('\n');

    if !task.description.is_empty() {
        result.push('\n');
        result.push_str(&task.description);
        result.push('\n');
    }

    Ok(result)
}

/// Split content into frontmatter and body
fn split_frontmatter(content: &str) -> Result<(String, String), FrontmatterError> {
    let content = content.trim();

    // Must start with delimiter
    if !content.starts_with(FRONTMATTER_DELIMITER) {
        return Err(FrontmatterError::MissingDelimiters);
    }

    // Find the closing delimiter
    let after_first = &content[FRONTMATTER_DELIMITER.len()..];
    let after_first = after_first.trim_start_matches('\n');

    if let Some(end_pos) = after_first.find(&format!("\n{}", FRONTMATTER_DELIMITER)) {
        let frontmatter = &after_first[..end_pos];
        let body_start = end_pos + 1 + FRONTMATTER_DELIMITER.len();
        let body = if body_start < after_first.len() {
            &after_first[body_start..]
        } else {
            ""
        };
        Ok((frontmatter.to_string(), body.to_string()))
    } else {
        Err(FrontmatterError::MissingDelimiters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::{Priority, TaskKind, TaskStatus};
    use chrono::NaiveDate;

    #[test]
    fn test_split_frontmatter() {
        let content = r#"---
id: 1
title: Test task
---

This is the body.
"#;
        let (frontmatter, body) = split_frontmatter(content).unwrap();
        assert!(frontmatter.contains("id: 1"));
        assert!(frontmatter.contains("title: Test task"));
        assert!(body.contains("This is the body."));
    }

    #[test]
    fn test_split_frontmatter_no_body() {
        let content = r#"---
id: 1
title: Test task
---"#;
        let (frontmatter, body) = split_frontmatter(content).unwrap();
        assert!(frontmatter.contains("id: 1"));
        assert!(body.is_empty());
    }

    #[test]
    fn test_split_frontmatter_missing_start() {
        let content = "No frontmatter here";
        assert!(split_frontmatter(content).is_err());
    }

    #[test]
    fn test_split_frontmatter_missing_end() {
        let content = r#"---
id: 1
title: Test task"#;
        assert!(split_frontmatter(content).is_err());
    }

    #[test]
    fn test_parse_task() {
        let content = r#"---
id: 1
title: Fix authentication bug
status: in-progress
priority: high
kind: task
tags:
  - auth
  - security
due: 2026-02-20
created: 2026-02-13T10:30:00Z
updated: 2026-02-13T10:30:00Z
---

This is the task description.
It can have multiple lines.
"#;
        let task = parse_task(content).unwrap();
        assert_eq!(task.id, 1);
        assert_eq!(task.title, "Fix authentication bug");
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.kind, TaskKind::Task);
        assert_eq!(task.tags, vec!["auth", "security"]);
        assert_eq!(
            task.due,
            Some(NaiveDate::from_ymd_opt(2026, 2, 20).unwrap())
        );
        assert!(task.description.contains("This is the task description."));
        assert!(task.description.contains("multiple lines"));
    }

    #[test]
    fn test_parse_task_minimal() {
        let content = r#"---
id: 1
title: Simple task
created: 2026-02-13T10:30:00Z
updated: 2026-02-13T10:30:00Z
---
"#;
        let task = parse_task(content).unwrap();
        assert_eq!(task.id, 1);
        assert_eq!(task.title, "Simple task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.kind, TaskKind::Task);
        assert!(task.tags.is_empty());
        assert!(task.due.is_none());
    }

    #[test]
    fn test_serialize_task() {
        let mut task = Task::new(1, TaskKind::Task, "Test task");
        task.priority = Priority::High;
        task.tags = vec!["test".to_string(), "example".to_string()];
        task.description = "Task description here.".to_string();

        let content = serialize_task(&task).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("id: 1"));
        assert!(content.contains("title: Test task"));
        assert!(content.contains("priority: high"));
        assert!(content.contains("- test"));
        assert!(content.contains("Task description here."));
    }

    #[test]
    fn test_roundtrip() {
        let mut task = Task::new(42, TaskKind::Idea, "New feature idea");
        task.priority = Priority::Low;
        task.tags = vec!["feature".to_string()];
        task.description = "Detailed description\nwith multiple lines.".to_string();

        let content = serialize_task(&task).unwrap();
        let parsed = parse_task(&content).unwrap();

        assert_eq!(task.id, parsed.id);
        assert_eq!(task.title, parsed.title);
        assert_eq!(task.priority, parsed.priority);
        assert_eq!(task.kind, parsed.kind);
        assert_eq!(task.tags, parsed.tags);
        assert_eq!(task.description, parsed.description);
    }
}
