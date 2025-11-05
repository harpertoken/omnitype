//! Utility functions and helpers for omnitype.

#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::empty_line_after_outer_attr)]

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Returns an iterator over all Python files in the given directory.

pub fn find_python_files<P: AsRef<Path>>(path: P) -> impl Iterator<Item = PathBuf> {

    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {

            let path = entry.path();

            path.is_file() && path.extension().is_some_and(|ext| ext == "py")
        })
        .map(|entry| entry.path().to_path_buf())
}

/// Converts a path to a module name.
/// Returns None if the path ends with a trailing slash (indicating a directory)
/// or has no file stem.

pub fn path_to_module_name(path: &Path) -> Option<String> {

    // Check if the path ends with a separator (indicates a directory)
    if path.to_str()?.ends_with(std::path::MAIN_SEPARATOR) || path.to_str()?.ends_with('/') {

        return None;
    }

    // Get the file stem (without extension)
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

#[cfg(test)]

mod tests {

    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]

    fn test_find_python_files() {

        let temp_dir = tempdir().unwrap();

        let dir_path = temp_dir.path();

        let _ = File::create(dir_path.join("test1.py")).unwrap();

        let _ = File::create(dir_path.join("test2.py")).unwrap();

        let _ = File::create(dir_path.join("not_python.txt")).unwrap();

        temp_dir.close().unwrap();
    }

    #[test]

    fn test_path_to_module_name() {

        assert_eq!(
            path_to_module_name(Path::new("/path/to/module.py")),
            Some("module".to_string())
        );

        assert_eq!(path_to_module_name(Path::new("module.py")), Some("module".to_string()));

        assert_eq!(path_to_module_name(Path::new("/path/to/dir/")), None);
    }
}
