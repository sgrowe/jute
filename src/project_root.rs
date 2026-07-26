use std::path::Path;

/// Walks up from `start` (inclusive) and returns the nearest ancestor that
/// contains a `.jute/` subdirectory.
///
/// `start` should be absolute — `Path::ancestors` on a relative path stops at
/// the empty path rather than continuing up to the filesystem root.
pub fn find_project_root(start: &Path) -> Option<&Path> {
    start.ancestors().find(|dir| dir.join(".jute").is_dir())
}
