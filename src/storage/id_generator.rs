//! Sequential ID generation for tasks

use std::path::Path;
use thiserror::Error;

/// Errors related to ID generation
#[derive(Debug, Error)]
pub enum IdError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid filename format: {0}")]
    InvalidFilename(String),
}

/// Generates sequential IDs for tasks
pub struct IdGenerator;

impl IdGenerator {
    /// Scan a directory and return the next available ID
    pub fn next_id(tasks_dir: &Path) -> Result<u64, IdError> {
        let max_id = Self::find_max_id(tasks_dir)?;
        Ok(max_id + 1)
    }

    /// Find the maximum ID in the tasks directory
    pub fn find_max_id(tasks_dir: &Path) -> Result<u64, IdError> {
        if !tasks_dir.exists() {
            return Ok(0);
        }

        let mut max_id: u64 = 0;

        for entry in std::fs::read_dir(tasks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "md")
                && let Some(id) = Self::extract_id_from_filename(&path)
            {
                max_id = max_id.max(id);
            }
        }

        Ok(max_id)
    }

    /// Extract ID from a task filename
    /// Expected format: {slug}-{id}.md (e.g., fix-auth-bug-001.md)
    pub fn extract_id_from_filename(path: &Path) -> Option<u64> {
        let stem = path.file_stem()?.to_str()?;

        // Find the last hyphen followed by digits
        if let Some(last_hyphen) = stem.rfind('-') {
            let id_part = &stem[last_hyphen + 1..];
            id_part.parse::<u64>().ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_extract_id_simple() {
        let path = Path::new("fix-auth-bug-001.md");
        assert_eq!(IdGenerator::extract_id_from_filename(path), Some(1));
    }

    #[test]
    fn test_extract_id_large_number() {
        let path = Path::new("test-task-999.md");
        assert_eq!(IdGenerator::extract_id_from_filename(path), Some(999));
    }

    #[test]
    fn test_extract_id_single_word() {
        let path = Path::new("test-42.md");
        assert_eq!(IdGenerator::extract_id_from_filename(path), Some(42));
    }

    #[test]
    fn test_extract_id_no_hyphen() {
        let path = Path::new("test.md");
        assert_eq!(IdGenerator::extract_id_from_filename(path), None);
    }

    #[test]
    fn test_extract_id_no_number() {
        let path = Path::new("test-abc.md");
        assert_eq!(IdGenerator::extract_id_from_filename(path), None);
    }

    #[test]
    fn test_next_id_empty_dir() {
        let temp = TempDir::new().unwrap();
        assert_eq!(IdGenerator::next_id(temp.path()).unwrap(), 1);
    }

    #[test]
    fn test_next_id_nonexistent_dir() {
        let path = Path::new("/nonexistent/path");
        assert_eq!(IdGenerator::next_id(path).unwrap(), 1);
    }

    #[test]
    fn test_next_id_with_files() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("task-001.md")).unwrap();
        File::create(temp.path().join("another-task-005.md")).unwrap();
        File::create(temp.path().join("third-task-003.md")).unwrap();

        assert_eq!(IdGenerator::next_id(temp.path()).unwrap(), 6);
    }

    #[test]
    fn test_next_id_ignores_non_md() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("task-001.md")).unwrap();
        File::create(temp.path().join("task-999.txt")).unwrap();

        assert_eq!(IdGenerator::next_id(temp.path()).unwrap(), 2);
    }

    #[test]
    fn test_find_max_id() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("a-1.md")).unwrap();
        File::create(temp.path().join("b-10.md")).unwrap();
        File::create(temp.path().join("c-5.md")).unwrap();

        assert_eq!(IdGenerator::find_max_id(temp.path()).unwrap(), 10);
    }
}
