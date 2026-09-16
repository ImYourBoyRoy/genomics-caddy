//! Small filesystem primitives for durable, same-directory application writes.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Write bytes through a same-directory temporary file and atomically publish them.
///
/// The destination is never removed as part of error handling. On filesystems
/// where replacement rename is unsupported, the original destination therefore
/// remains intact and the temporary file is cleaned up.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|candidate| !candidate.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "Atomic write path must include a valid file name".to_string())?;
    let temp_name = format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let temp_path = parent.join(temp_name);

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| format!("Could not create temporary file: {error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("Could not write temporary file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not sync temporary file: {error}"))?;
        fs::rename(&temp_path, path)
            .map_err(|error| format!("Could not publish atomic file: {error}"))?;
        Ok::<(), String>(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::atomic_write;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn atomic_write_publishes_complete_content_and_replaces_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "dna_tools_atomic_write_test_{}_{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("report.json");
        fs::write(&path, b"old").expect("seed destination");

        atomic_write(&path, b"new complete content").expect("publish atomic content");
        assert_eq!(
            fs::read(&path).expect("read destination"),
            b"new complete content"
        );

        let _ = fs::remove_dir_all(dir);
    }
}
