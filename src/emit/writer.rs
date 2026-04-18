use crate::codegen::tree::OutputFile;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub fn write_files(base: &Path, files: &[OutputFile]) -> io::Result<()> {
    // Validate first (cheap, serial) so a traversal attempt fails before we touch disk.
    for file in files {
        if file.path.contains("..") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("path traversal rejected: {}", file.path),
            ));
        }
    }

    // Pre-create parent directories once each. `fs::create_dir_all` is technically
    // concurrent-safe, but calling it N times wastes syscalls on the common parent.
    let created: Mutex<HashSet<PathBuf>> = Mutex::new(HashSet::new());
    for file in files {
        let full_path = base.join(&file.path);
        if let Some(parent) = full_path.parent() {
            let mut set = created.lock().unwrap();
            if set.insert(parent.to_path_buf()) {
                fs::create_dir_all(parent)?;
            }
        }
    }

    files.par_iter().try_for_each(|file| -> io::Result<()> {
        let full_path = base.join(&file.path);
        if let Some(ref bytes) = file.binary {
            fs::write(&full_path, bytes)?;
        } else {
            fs::write(&full_path, &file.content)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::tree::OutputFile;
    use std::fs;

    #[test]
    fn test_write_files_creates_dirs_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        let files = vec![
            OutputFile {
                path: "Card/Card.tsx".into(),
                content: "export function Card() {}".into(),
                binary: None,
            },
            OutputFile {
                path: "Card/index.ts".into(),
                content: "export { Card } from './Card';".into(),
                binary: None,
            },
        ];
        write_files(&base, &files).unwrap();
        assert!(base.join("Card/Card.tsx").exists());
        assert!(base.join("Card/index.ts").exists());
        let content = fs::read_to_string(base.join("Card/Card.tsx")).unwrap();
        assert_eq!(content, "export function Card() {}");
    }

    #[test]
    fn test_write_binary_file() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        let files = vec![OutputFile {
            path: "assets/image.png".into(),
            content: String::new(),
            binary: Some(vec![0x89, 0x50, 0x4E, 0x47]),
        }];
        write_files(&base, &files).unwrap();
        let bytes = fs::read(base.join("assets/image.png")).unwrap();
        assert_eq!(bytes, vec![0x89, 0x50, 0x4E, 0x47]);
    }
}
