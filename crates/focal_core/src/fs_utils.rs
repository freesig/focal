use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pathdiff::diff_paths;
use uuid::Uuid;

use crate::error::GraphError;

pub(crate) const ROOTS_DIR: &str = "roots";
pub(crate) const NODE_FILE: &str = "node.md";
pub(crate) const CHILDREN_DIR: &str = "children";
const MAX_SLUG_BYTES: usize = 80;

pub(crate) fn now_unix() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

pub(crate) fn validate_node_id(id: &str) -> Result<(), GraphError> {
    let valid_shape = id.len() == 36
        && id.char_indices().all(|(index, ch)| match index {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_digit() || ('a'..='f').contains(&ch),
        });

    if !valid_shape || Uuid::parse_str(id).is_err() {
        return Err(GraphError::InvalidNodeId(id.to_string()));
    }

    Ok(())
}

pub(crate) fn validate_title(title: &str) -> Result<(), GraphError> {
    if title.trim().is_empty() || title.chars().any(char::is_control) {
        return Err(GraphError::InvalidTitle);
    }
    Ok(())
}

pub(crate) fn generate_node_id() -> String {
    Uuid::new_v4().to_string()
}

pub(crate) fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;

    for ch in title.trim().chars() {
        if slug.len() >= MAX_SLUG_BYTES {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            slug.push('-');
            previous_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "node".to_string()
    } else {
        slug.to_string()
    }
}

pub(crate) fn unique_node_path(
    container: &Path,
    title: &str,
    id: &str,
) -> Result<PathBuf, GraphError> {
    let slug = slugify(title);
    let mut suffix = 1usize;

    loop {
        let name = if suffix == 1 {
            format!("{slug}--{id}")
        } else {
            format!("{slug}-{suffix}--{id}")
        };
        let candidate = container.join(name);
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Err(error) => return Err(GraphError::Io(error)),
        }
        suffix = suffix
            .checked_add(1)
            .ok_or_else(|| GraphError::AliasConflict(container.to_path_buf()))?;
    }
}

pub(crate) fn node_id_from_dir_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    let (_, id) = name.rsplit_once("--")?;
    if validate_node_id(id).is_ok() {
        Some(id.to_string())
    } else {
        None
    }
}

pub(crate) fn roots_path(root: &Path) -> PathBuf {
    root.join(ROOTS_DIR)
}

pub(crate) fn node_file_path(node_dir: &Path) -> PathBuf {
    node_dir.join(NODE_FILE)
}

pub(crate) fn children_path(node_dir: &Path) -> PathBuf {
    node_dir.join(CHILDREN_DIR)
}

pub(crate) fn canonicalize_existing(path: &Path) -> Result<PathBuf, GraphError> {
    path.canonicalize().map_err(GraphError::Io)
}

pub(crate) fn absolute_path(path: &Path) -> Result<PathBuf, GraphError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

pub(crate) fn ensure_existing_path_inside(root: &Path, path: &Path) -> Result<PathBuf, GraphError> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) {
        return Err(GraphError::PermissionDenied(path.to_path_buf()));
    }
    Ok(canonical)
}

pub(crate) fn ensure_parent_inside(root: &Path, path: &Path) -> Result<(), GraphError> {
    let Some(parent) = path.parent() else {
        return Err(GraphError::PermissionDenied(path.to_path_buf()));
    };
    let parent = parent.canonicalize()?;
    if !parent.starts_with(root) {
        return Err(GraphError::PermissionDenied(path.to_path_buf()));
    }
    Ok(())
}

pub(crate) fn safe_remove_file(root: &Path, path: &Path) -> Result<(), GraphError> {
    ensure_parent_inside(root, path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(GraphError::Io(error)),
    }
}

pub(crate) fn safe_remove_dir_all(root: &Path, path: &Path) -> Result<(), GraphError> {
    match ensure_existing_path_inside(root, path) {
        Ok(_) => {}
        Err(GraphError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(());
        }
        Err(error) => return Err(error),
    }
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(GraphError::Io(error)),
    }
}

pub(crate) fn safe_rename(root: &Path, from: &Path, to: &Path) -> Result<(), GraphError> {
    ensure_existing_path_inside(root, from)?;
    ensure_parent_inside(root, to)?;
    fs::rename(from, to).map_err(GraphError::Io)
}

pub(crate) fn create_relative_dir_symlink(
    root: &Path,
    target_dir: &Path,
    link_path: &Path,
) -> Result<(), GraphError> {
    ensure_existing_path_inside(root, target_dir)?;
    ensure_parent_inside(root, link_path)?;
    let Some(link_parent) = link_path.parent() else {
        return Err(GraphError::AliasConflict(link_path.to_path_buf()));
    };
    let target = match diff_paths(target_dir, link_parent) {
        Some(path) => path,
        None => target_dir.to_path_buf(),
    };
    create_dir_symlink(&target, link_path)
}

pub(crate) fn write_file_atomically(
    root: &Path,
    path: &Path,
    contents: &str,
) -> Result<(), GraphError> {
    ensure_parent_inside(root, path)?;
    let parent = path
        .parent()
        .ok_or_else(|| GraphError::PermissionDenied(path.to_path_buf()))?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| GraphError::PermissionDenied(path.to_path_buf()))?;
    let temp_path = parent.join(format!(".{file_name}.{}.tmp", generate_node_id()));

    let write_result = (|| -> Result<(), std::io::Error> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        Ok(())
    })();

    match write_result {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(GraphError::Io(error))
        }
    }
}

#[cfg(unix)]
fn create_dir_symlink(target: &Path, link_path: &Path) -> Result<(), GraphError> {
    std::os::unix::fs::symlink(target, link_path).map_err(GraphError::Io)
}

#[cfg(not(unix))]
fn create_dir_symlink(_target: &Path, _link_path: &Path) -> Result<(), GraphError> {
    Err(GraphError::SymlinkUnsupported(
        "directory symlinks require a Unix-like platform in this version".to_string(),
    ))
}

pub(crate) fn resolve_symlink_path(link_path: &Path) -> Result<PathBuf, GraphError> {
    let target = fs::read_link(link_path)?;
    if target.is_absolute() {
        Ok(target)
    } else {
        let parent = link_path
            .parent()
            .ok_or_else(|| GraphError::BrokenSymlink(link_path.to_path_buf()))?;
        Ok(parent.join(target))
    }
}

pub(crate) fn is_path_inside_any(path: &Path, candidates: &[PathBuf]) -> bool {
    candidates
        .iter()
        .any(|candidate| path == candidate || path.starts_with(candidate))
}

pub(crate) fn path_sort_key(path: &Path) -> String {
    path.as_os_str().to_string_lossy().into_owned()
}

pub(crate) fn has_node_dir_suffix(path: &Path, id: &str) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .and_then(|name| name.rsplit_once("--").map(|(_, suffix)| suffix))
        .is_some_and(|suffix| suffix == id)
}
