//! Git operations for task management

use git2::Repository;
use std::path::Path;
use thiserror::Error;

/// Errors related to git operations
#[derive(Debug, Error)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Not in a git repository")]
    NotInRepo,
    #[error("No HEAD commit found")]
    NoHead,
}

/// Git operations helper
pub struct GitOperations;

impl GitOperations {
    /// Check if a path is inside a git repository
    pub fn is_in_repo(path: &Path) -> bool {
        Repository::discover(path).is_ok()
    }

    /// Get the repository for a path
    pub fn repo(path: &Path) -> Result<Repository, GitError> {
        Ok(Repository::discover(path)?)
    }

    /// Get the root directory of the git repository
    pub fn repo_root(path: &Path) -> Result<std::path::PathBuf, GitError> {
        let repo = Repository::discover(path)?;
        repo.workdir()
            .map(|p| p.to_path_buf())
            .ok_or(GitError::NotInRepo)
    }

    /// Get the current HEAD commit hash (short form)
    pub fn head_commit_short(path: &Path) -> Result<String, GitError> {
        let repo = Repository::discover(path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let id = commit.id();
        Ok(format!("{:.7}", id))
    }

    /// Get the current HEAD commit hash (full form)
    pub fn head_commit(path: &Path) -> Result<String, GitError> {
        let repo = Repository::discover(path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Get the current HEAD commit hash if available, or None if not in a repo or no commits
    pub fn head_commit_optional(path: &Path) -> Option<String> {
        Self::head_commit_short(path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_git_repo() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        temp
    }

    #[test]
    fn test_is_in_repo() {
        let temp = setup_git_repo();
        assert!(GitOperations::is_in_repo(temp.path()));

        let non_repo = TempDir::new().unwrap();
        assert!(!GitOperations::is_in_repo(non_repo.path()));
    }

    #[test]
    fn test_repo_root() {
        let temp = setup_git_repo();
        let subdir = temp.path().join("src");
        std::fs::create_dir(&subdir).unwrap();

        let root = GitOperations::repo_root(&subdir).unwrap();
        assert_eq!(
            root.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_head_commit() {
        let temp = setup_git_repo();

        // Create a file and commit
        std::fs::write(temp.path().join("test.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let commit = GitOperations::head_commit_short(temp.path()).unwrap();
        assert_eq!(commit.len(), 7);

        let full_commit = GitOperations::head_commit(temp.path()).unwrap();
        assert_eq!(full_commit.len(), 40);
    }

    #[test]
    fn test_head_commit_optional() {
        let temp = setup_git_repo();

        // No commits yet
        assert!(GitOperations::head_commit_optional(temp.path()).is_none());

        // Create a commit
        std::fs::write(temp.path().join("test.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        assert!(GitOperations::head_commit_optional(temp.path()).is_some());
    }
}
