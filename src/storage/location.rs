//! Task directory location detection and management

use std::path::PathBuf;
use thiserror::Error;

/// Task directory name
const TASKS_DIR: &str = ".tasks";

/// Errors related to task location
#[derive(Debug, Error)]
pub enum TaskLocationError {
    #[error("Not in a git repository")]
    NotInGitRepo,
    #[error("Task directory does not exist: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("Failed to access home directory")]
    NoHomeDirectory,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Represents where tasks are stored
#[derive(Debug, Clone)]
pub struct TaskLocation {
    /// The root directory containing .tasks
    pub root: PathBuf,
    /// The .tasks directory itself
    pub tasks_dir: PathBuf,
    /// Whether this is a global location (~/.tasks)
    pub is_global: bool,
}

impl TaskLocation {
    /// Find the project task location (in git repo root)
    pub fn find_project() -> Result<Self, TaskLocationError> {
        let current = std::env::current_dir()?;
        Self::find_project_from(&current)
    }

    /// Find the project task location starting from a specific directory
    pub fn find_project_from(start: &std::path::Path) -> Result<Self, TaskLocationError> {
        // Walk up the directory tree looking for .git
        let mut current = start.to_path_buf();
        loop {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                let tasks_dir = current.join(TASKS_DIR);
                return Ok(TaskLocation {
                    root: current,
                    tasks_dir,
                    is_global: false,
                });
            }

            if !current.pop() {
                return Err(TaskLocationError::NotInGitRepo);
            }
        }
    }

    /// Get the global task location (~/.tasks)
    pub fn global() -> Result<Self, TaskLocationError> {
        let home = dirs::home_dir().ok_or(TaskLocationError::NoHomeDirectory)?;
        let tasks_dir = home.join(TASKS_DIR);
        Ok(TaskLocation {
            root: home,
            tasks_dir,
            is_global: true,
        })
    }

    /// Check if the tasks directory exists
    pub fn exists(&self) -> bool {
        self.tasks_dir.exists()
    }

    /// Create the tasks directory if it doesn't exist
    pub fn ensure_exists(&self) -> Result<(), TaskLocationError> {
        if !self.tasks_dir.exists() {
            std::fs::create_dir_all(&self.tasks_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_location() {
        let loc = TaskLocation::global().unwrap();
        assert!(loc.is_global);
        assert!(loc.tasks_dir.ends_with(".tasks"));
    }

    #[test]
    fn test_find_project_from_git_root() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();

        let loc = TaskLocation::find_project_from(temp.path()).unwrap();
        assert!(!loc.is_global);
        assert_eq!(loc.root, temp.path());
        assert_eq!(loc.tasks_dir, temp.path().join(".tasks"));
    }

    #[test]
    fn test_find_project_from_subdirectory() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        let subdir = temp.path().join("src").join("nested");
        std::fs::create_dir_all(&subdir).unwrap();

        let loc = TaskLocation::find_project_from(&subdir).unwrap();
        assert!(!loc.is_global);
        assert_eq!(loc.root, temp.path());
    }

    #[test]
    fn test_find_project_no_git() {
        let temp = TempDir::new().unwrap();
        assert!(TaskLocation::find_project_from(temp.path()).is_err());
    }

    #[test]
    fn test_ensure_exists() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();

        let loc = TaskLocation::find_project_from(temp.path()).unwrap();
        assert!(!loc.exists());

        loc.ensure_exists().unwrap();
        assert!(loc.exists());
    }
}
