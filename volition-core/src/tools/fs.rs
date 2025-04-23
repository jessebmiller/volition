// volition-agent-core/src/tools/fs.rs

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tracing::info;

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

pub async fn read_file(relative_path: &str, working_dir: &Path) -> Result<String, String> {
    let path = working_dir.join(relative_path);
    info!("Reading file: {}", path.display());
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

pub async fn write_file(relative_path: &str, content: &str, working_dir: &Path) -> Result<String, String> {
    let path = working_dir.join(relative_path);
    info!("Writing file: {}", path.display());
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(format!("Successfully wrote to file: {}", relative_path))
}

pub fn list_directory_contents(path: &str) -> Result<Vec<FileInfo>, String> {
    fn list_recursive(base_path: &Path, current_path: &Path, files: &mut Vec<FileInfo>) -> Result<(), String> {
        let entries = match fs::read_dir(current_path) {
            Ok(entries) => entries,
            Err(e) => return Err(format!("Failed to read directory: {}", e)),
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("Error reading directory entry: {}", e);
                    continue;
                }
            };

            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    eprintln!("Error reading metadata: {}", e);
                    continue;
                }
            };

            let file_type = if metadata.is_dir() {
                "directory"
            } else if metadata.is_file() {
                "file"
            } else if metadata.is_symlink() {
                "symlink"
            } else {
                "unknown"
            };

            let size = if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            };

            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs());

            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(base_path).unwrap_or(&entry_path);
            let name = relative_path.to_string_lossy().into_owned();
            let full_path = entry_path.to_string_lossy().into_owned();

            let file_info = FileInfo {
                name,
                path: full_path,
                file_type: file_type.to_string(),
                size,
                modified,
            };

            files.push(file_info);

            if metadata.is_dir() {
                list_recursive(base_path, &entry_path, files)?;
            }
        }
        Ok(())
    }

    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    let mut files = Vec::new();
    list_recursive(path, path, &mut files)?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    fn sort_lines(text: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();
        lines
    }

    #[test]
    fn test_fs_list_basic() -> Result<(), String> {
        let dir = tempdir().map_err(|e| e.to_string())?;
        let wd = dir.path();
        File::create(wd.join("f1.txt")).map_err(|e| e.to_string())?;
        fs::create_dir(wd.join("sd")).map_err(|e| e.to_string())?;
        File::create(wd.join("sd/f2.txt")).map_err(|e| e.to_string())?;
        let output = list_directory_contents(wd.to_str().unwrap())?;
        let names: Vec<String> = output.iter().map(|f| f.name.clone()).collect();
        assert_eq!(sort_lines(&names.join("\n")), sort_lines("f1.txt\nsd"));
        Ok(())
    }

    #[test]
    fn test_fs_list_depth() -> Result<(), String> {
        let dir = tempdir().map_err(|e| e.to_string())?;
        let wd = dir.path();
        File::create(wd.join("f1.txt")).map_err(|e| e.to_string())?;
        fs::create_dir(wd.join("sd")).map_err(|e| e.to_string())?;
        File::create(wd.join("sd/f2.txt")).map_err(|e| e.to_string())?;
        let output = list_directory_contents(wd.to_str().unwrap())?;
        let names: Vec<String> = output.iter().map(|f| f.name.clone()).collect();
        let expected = format!("f1.txt\nsd\nsd{}f2.txt", std::path::MAIN_SEPARATOR);
        assert_eq!(sort_lines(&names.join("\n")), sort_lines(&expected));
        Ok(())
    }
}
