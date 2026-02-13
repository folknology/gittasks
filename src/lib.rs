//! gittask - Git-versioned task management using Markdown files
//!
//! This library provides the core functionality for managing tasks stored as
//! Markdown files with YAML frontmatter in a git repository.

pub mod cli;
pub mod git;
pub mod mcp;
pub mod models;
pub mod storage;

pub use models::{Priority, Task, TaskKind, TaskStatus};
pub use storage::{FileStore, ProjectRegistry, ProjectStatus, TaskFilter, TaskLocation, TaskStats};
