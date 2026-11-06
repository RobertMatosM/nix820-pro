//! XDG user directories. Never derives writable paths from the build location.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub config: PathBuf,
    pub data: PathBuf,
}
impl AppPaths {
    pub fn previous_name(&self) -> Self {
        Self {
            config: self.config.with_file_name("keyboard-studio"),
            data: self.data.with_file_name("keyboard-studio"),
        }
    }

    pub fn discover() -> Result<Self, String> {
        Self::resolve(
            std::env::var_os("HOME").as_deref().map(Path::new),
            std::env::var_os("XDG_CONFIG_HOME")
                .as_deref()
                .map(Path::new),
            std::env::var_os("XDG_DATA_HOME").as_deref().map(Path::new),
        )
    }
    fn resolve(
        home: Option<&Path>,
        config: Option<&Path>,
        data: Option<&Path>,
    ) -> Result<Self, String> {
        let base = |value: Option<&Path>, fallback: &str| -> Result<PathBuf, String> {
            if let Some(path) = value.filter(|path| path.is_absolute()) {
                return Ok(path.to_path_buf());
            }
            home.filter(|path| path.is_absolute())
                .map(|path| path.join(fallback))
                .ok_or_else(|| {
                    "Set HOME or absolute XDG_CONFIG_HOME and XDG_DATA_HOME directories.".into()
                })
        };
        Ok(Self {
            config: base(config, ".config")?.join("nix820-pro"),
            data: base(data, ".local/share")?.join("nix820-pro"),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_paths_honor_absolute_xdg_values_and_ignore_relative_or_empty_overrides() {
        let home = Some(Path::new("/tmp/ks-home"));
        let paths =
            AppPaths::resolve(home, Some(Path::new("relative")), Some(Path::new(""))).unwrap();
        assert_eq!(paths.config, Path::new("/tmp/ks-home/.config/nix820-pro"));
        assert_eq!(
            paths.data,
            Path::new("/tmp/ks-home/.local/share/nix820-pro")
        );
        let paths = AppPaths::resolve(
            None,
            Some(Path::new("/tmp/config")),
            Some(Path::new("/tmp/data")),
        )
        .unwrap();
        assert_eq!(paths.config, Path::new("/tmp/config/nix820-pro"));
        assert_eq!(paths.data, Path::new("/tmp/data/nix820-pro"));
        assert!(AppPaths::resolve(None, None, None).is_err());
    }
}

/// Read an existing user file. Only a missing file means "use defaults".
pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Could not read {}: {error}. The file was left untouched.",
            path.display()
        )),
    }
}

/// Publish a complete file atomically using a unique, private staging file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().ok_or("Missing save directory")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_file_name(format!(
        ".{}.save-{}-{}.tmp",
        path.file_name()
            .ok_or("Missing save filename")?
            .to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })()
    .map_err(|error| error.to_string());
    drop(file);
    let _ = std::fs::remove_file(&temporary);
    result
}

#[cfg(test)]
mod atomic_tests {
    use super::*;
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };
    struct Running(Arc<AtomicUsize>);
    impl Drop for Running {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn concurrent_saves_always_publish_complete_json_and_leave_no_staging_files() {
        let root = std::env::temp_dir().join(format!("nix820-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("profiles.json");
        write_atomic(&path, b"{\"writer\":0}").unwrap();
        let barrier = Arc::new(Barrier::new(5));
        let remaining = Arc::new(AtomicUsize::new(4));
        std::thread::scope(|scope| {
            for writer in 1..=4 {
                let path = &path;
                let barrier = barrier.clone();
                let remaining = remaining.clone();
                scope.spawn(move || {
                    let _running = Running(remaining);
                    barrier.wait();
                    for _ in 0..12 {
                        let bytes = serde_json::to_vec(
                            &serde_json::json!({"writer":writer,"data":"x".repeat(16000)}),
                        )
                        .unwrap();
                        write_atomic(path, &bytes).unwrap();
                    }
                });
            }
            barrier.wait();
            while remaining.load(Ordering::SeqCst) > 0 {
                let value: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert!(value["writer"].as_u64().unwrap() <= 4);
            }
        });
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
