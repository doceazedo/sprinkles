use std::env;
use std::path::{Path, PathBuf};

pub const MAX_DISPLAY_PATH_LEN: usize = 40;

pub fn truncate_path(path: &str, max_len: usize) -> String {
    let path = simplify_path(Path::new(path));
    if path.len() <= max_len {
        return path;
    }

    let sep = '/';
    let segments: Vec<&str> = path.split(sep).collect();
    if segments.len() <= 2 {
        return path;
    }

    let last = segments[segments.len() - 1];
    let ellipsis = "...";

    let mut prefix = String::new();
    for &seg in &segments[..segments.len() - 1] {
        let next_prefix = if prefix.is_empty() {
            seg.to_string()
        } else {
            format!("{prefix}{sep}{seg}")
        };
        if format!("{next_prefix}{sep}{ellipsis}{sep}{last}").len() > max_len {
            break;
        }
        prefix = next_prefix;
    }

    if prefix.is_empty() {
        format!("{ellipsis}{sep}{last}")
    } else {
        format!("{prefix}{sep}{ellipsis}{sep}{last}")
    }
}

pub fn simplify_path(path: &Path) -> String {
    #[cfg(unix)]
    let home = env::var_os("HOME").map(PathBuf::from);
    #[cfg(not(unix))]
    let home = env::var_os("USERPROFILE").map(PathBuf::from);
    if let Some(home_path) = home {
        if let Ok(relative) = path.strip_prefix(&home_path) {
            return format!("~/{}", relative.to_string_lossy());
        }
    }
    path.to_string_lossy().to_string()
}
