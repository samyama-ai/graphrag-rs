use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

pub struct FileEntry {
    pub path: String,
    pub content: String,
}

/// Walk a directory (or single file) and read supported file types.
pub fn walk_and_read(path: &str) -> Result<Vec<FileEntry>> {
    let path = Path::new(path);
    let mut entries = Vec::new();

    if path.is_file() {
        if let Some(content) = read_file(path)? {
            entries.push(FileEntry {
                path: path.display().to_string(),
                content,
            });
        }
        return Ok(entries);
    }

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(content) = read_file(entry.path())? {
            entries.push(FileEntry {
                path: entry.path().display().to_string(),
                content,
            });
        }
    }

    Ok(entries)
}

/// Read a file if it has a supported extension. Returns None for unsupported types.
fn read_file(path: &Path) -> Result<Option<String>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" | "md" | "csv" | "json" | "rst" | "adoc" => {
            let content = std::fs::read_to_string(path)?;
            if content.trim().is_empty() {
                return Ok(None);
            }
            Ok(Some(content))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.md");
        fs::write(&file, "# Hello\nSome content").unwrap();

        let entries = walk_and_read(file.to_str().unwrap()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].content.contains("Hello"));
    }

    #[test]
    fn test_walk_directory() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "text file").unwrap();
        fs::write(dir.path().join("b.md"), "markdown file").unwrap();
        fs::write(dir.path().join("c.rs"), "fn main() {}").unwrap(); // unsupported

        let entries = walk_and_read(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_skip_empty_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.txt"), "   ").unwrap();

        let entries = walk_and_read(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(entries.len(), 0);
    }
}
