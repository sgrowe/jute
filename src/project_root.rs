use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectRoot<'a> {
    root: &'a Path,
}

impl<'a> ProjectRoot<'a> {
    /// Walks up from `start` (inclusive) and returns the nearest ancestor that
    /// contains a `.jute/` subdirectory.
    ///
    /// `start` should be absolute — `Path::ancestors` on a relative path stops at
    /// the empty path rather than continuing up to the filesystem root.
    pub fn find_project_root_starting_from(cwd: &'a Path) -> anyhow::Result<Self> {
        cwd.ancestors()
            .find(|dir| dir.join(".jute").is_dir())
            .map(|root| ProjectRoot { root })
            .ok_or_else(|| anyhow!("no .jute folder found in {} or any parent", cwd.display()))
    }

    fn jute_dir(&self) -> PathBuf {
        self.root.join(".jute")
    }

    pub fn read_tasks_file(&self) -> anyhow::Result<String> {
        let contents = read_to_string(self.jute_dir().join("tasks.jute"))?;

        Ok(contents)
    }
}
