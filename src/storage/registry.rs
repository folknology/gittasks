//! Project registry for aggregating tasks across multiple projects

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::storage::location::TaskLocation;
use crate::storage::{FileStore, TaskFilter};

/// Registry file name within the global tasks directory
const REGISTRY_FILE: &str = ".projects";

/// Errors related to the project registry
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Failed to access home directory")]
    NoHomeDirectory,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Status information for a registered project
#[derive(Debug, Clone)]
pub struct ProjectStatus {
    /// Project path
    pub path: PathBuf,
    /// Project name (directory name)
    pub name: String,
    /// Whether the path exists
    pub exists: bool,
    /// Whether the .tasks directory exists
    pub has_tasks_dir: bool,
    /// Number of open tasks (pending + in-progress)
    pub open_tasks: usize,
    /// Total number of tasks
    pub total_tasks: usize,
}

impl ProjectStatus {
    /// Create a new ProjectStatus by inspecting the project path
    pub fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let exists = path.exists();
        let tasks_dir = path.join(".tasks");
        let has_tasks_dir = tasks_dir.exists();

        let (open_tasks, total_tasks) = if has_tasks_dir {
            if let Ok(location) = TaskLocation::find_project_from(path) {
                let store = FileStore::new(location);
                if let Ok(tasks) = store.list(&TaskFilter {
                    include_archived: true,
                    ..Default::default()
                }) {
                    let open = tasks.iter().filter(|t| t.is_open()).count();
                    (open, tasks.len())
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        ProjectStatus {
            path: path.to_path_buf(),
            name,
            exists,
            has_tasks_dir,
            open_tasks,
            total_tasks,
        }
    }
}

/// Registry of projects for aggregated task views
#[derive(Debug)]
pub struct ProjectRegistry {
    /// Path to the registry file (~/.tasks/.projects)
    registry_path: PathBuf,
    /// Registered project paths
    projects: HashSet<PathBuf>,
}

impl ProjectRegistry {
    /// Load the registry from the default location (~/.tasks/.projects)
    pub fn load() -> Result<Self, RegistryError> {
        let home = dirs::home_dir().ok_or(RegistryError::NoHomeDirectory)?;
        let registry_path = home.join(".tasks").join(REGISTRY_FILE);
        Self::load_from(&registry_path)
    }

    /// Load the registry from a specific path
    pub fn load_from(path: &Path) -> Result<Self, RegistryError> {
        let projects = if path.exists() {
            let content = fs::read_to_string(path)?;
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| PathBuf::from(line.trim()))
                .collect()
        } else {
            HashSet::new()
        };

        Ok(ProjectRegistry {
            registry_path: path.to_path_buf(),
            projects,
        })
    }

    /// Save the registry to disk
    pub fn save(&self) -> Result<(), RegistryError> {
        // Ensure parent directory exists
        if let Some(parent) = self.registry_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content: String = self
            .projects
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Add trailing newline if there's content
        let content = if content.is_empty() {
            content
        } else {
            format!("{}\n", content)
        };

        fs::write(&self.registry_path, content)?;
        Ok(())
    }

    /// Register a project path (idempotent)
    pub fn link(&mut self, path: &Path) -> Result<bool, RegistryError> {
        let canonical = if path.exists() {
            path.canonicalize()?
        } else {
            path.to_path_buf()
        };

        let inserted = self.projects.insert(canonical);
        if inserted {
            self.save()?;
        }
        Ok(inserted)
    }

    /// Unregister a project path (idempotent)
    pub fn unlink(&mut self, path: &Path) -> Result<bool, RegistryError> {
        // Try both the path as-is and canonicalized
        let removed = self.projects.remove(path)
            || path
                .canonicalize()
                .map(|c| self.projects.remove(&c))
                .unwrap_or(false);

        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Get all registered project paths
    pub fn projects(&self) -> &HashSet<PathBuf> {
        &self.projects
    }

    /// Check if registry has any projects
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    /// Get the number of registered projects
    pub fn len(&self) -> usize {
        self.projects.len()
    }

    /// Get status information for all registered projects
    pub fn project_statuses(&self) -> Vec<ProjectStatus> {
        let mut statuses: Vec<_> = self
            .projects
            .iter()
            .map(|p| ProjectStatus::from_path(p))
            .collect();

        // Sort by name
        statuses.sort_by(|a, b| a.name.cmp(&b.name));
        statuses
    }

    /// Find a project by name (case-insensitive prefix match)
    pub fn find_project(&self, name: &str) -> Option<PathBuf> {
        let name_lower = name.to_lowercase();

        // First try exact match
        for path in &self.projects {
            if let Some(dir_name) = path.file_name()
                && dir_name.to_string_lossy().to_lowercase() == name_lower
            {
                return Some(path.clone());
            }
        }

        // Then try prefix match
        let mut matches: Vec<_> = self
            .projects
            .iter()
            .filter(|path| {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase().starts_with(&name_lower))
                    .unwrap_or(false)
            })
            .collect();

        if matches.len() == 1 {
            Some(matches.pop()?.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_empty_registry() {
        let temp = TempDir::new().unwrap();
        let registry_path = temp.path().join(".projects");

        let registry = ProjectRegistry::load_from(&registry_path).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_link_project() {
        let temp = TempDir::new().unwrap();
        let registry_path = temp.path().join(".projects");

        let mut registry = ProjectRegistry::load_from(&registry_path).unwrap();

        let project_path = temp.path().join("myproject");
        fs::create_dir(&project_path).unwrap();

        let inserted = registry.link(&project_path).unwrap();
        assert!(inserted);
        assert_eq!(registry.len(), 1);

        // Idempotent - linking again returns false
        let inserted = registry.link(&project_path).unwrap();
        assert!(!inserted);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_unlink_project() {
        let temp = TempDir::new().unwrap();
        let registry_path = temp.path().join(".projects");

        let mut registry = ProjectRegistry::load_from(&registry_path).unwrap();

        let project_path = temp.path().join("myproject");
        fs::create_dir(&project_path).unwrap();

        registry.link(&project_path).unwrap();
        assert_eq!(registry.len(), 1);

        let removed = registry.unlink(&project_path).unwrap();
        assert!(removed);
        assert!(registry.is_empty());

        // Idempotent - unlinking again returns false
        let removed = registry.unlink(&project_path).unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_save_and_load() {
        let temp = TempDir::new().unwrap();
        let registry_path = temp.path().join(".tasks").join(".projects");

        let project1 = temp.path().join("project1");
        let project2 = temp.path().join("project2");
        fs::create_dir(&project1).unwrap();
        fs::create_dir(&project2).unwrap();

        {
            let mut registry = ProjectRegistry::load_from(&registry_path).unwrap();
            registry.link(&project1).unwrap();
            registry.link(&project2).unwrap();
        }

        // Load again
        let registry = ProjectRegistry::load_from(&registry_path).unwrap();
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_find_project() {
        let temp = TempDir::new().unwrap();
        let registry_path = temp.path().join(".projects");

        let mut registry = ProjectRegistry::load_from(&registry_path).unwrap();

        let gittask = temp.path().join("gittask");
        let brooklyn = temp.path().join("brooklyn");
        fs::create_dir(&gittask).unwrap();
        fs::create_dir(&brooklyn).unwrap();

        registry.link(&gittask).unwrap();
        registry.link(&brooklyn).unwrap();

        // Exact match
        assert!(registry.find_project("gittask").is_some());
        assert!(registry.find_project("brooklyn").is_some());

        // Case insensitive
        assert!(registry.find_project("GitTask").is_some());

        // Prefix match
        assert!(registry.find_project("git").is_some());
        assert!(registry.find_project("brook").is_some());

        // No match
        assert!(registry.find_project("nonexistent").is_none());
    }

    #[test]
    fn test_project_status() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("myproject");
        fs::create_dir(&project_path).unwrap();

        let status = ProjectStatus::from_path(&project_path);
        assert_eq!(status.name, "myproject");
        assert!(status.exists);
        assert!(!status.has_tasks_dir);
        assert_eq!(status.open_tasks, 0);
        assert_eq!(status.total_tasks, 0);
    }
}
