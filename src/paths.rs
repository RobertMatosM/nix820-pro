//! XDG user directories. Never derives writable paths from the build location.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub config: PathBuf,
    pub data: PathBuf,
}
impl AppPaths {
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
            config: base(config, ".config")?.join("keyboard-studio"),
            data: base(data, ".local/share")?.join("keyboard-studio"),
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
        assert_eq!(
            paths.config,
            Path::new("/tmp/ks-home/.config/keyboard-studio")
        );
        assert_eq!(
            paths.data,
            Path::new("/tmp/ks-home/.local/share/keyboard-studio")
        );
        let paths = AppPaths::resolve(
            None,
            Some(Path::new("/tmp/config")),
            Some(Path::new("/tmp/data")),
        )
        .unwrap();
        assert_eq!(paths.config, Path::new("/tmp/config/keyboard-studio"));
        assert_eq!(paths.data, Path::new("/tmp/data/keyboard-studio"));
        assert!(AppPaths::resolve(None, None, None).is_err());
    }
}
