use std::{
    env,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use crate::error::{Context, JuteError, JuteResult};

/// The tasks file for the project the current directory belongs to.
pub fn find_and_read_tasks_file() -> JuteResult<String> {
    let cwd = env::current_dir().context("failed to determine the current directory")?;
    let project_root = ProjectRoot::find_project_root_starting_from(&cwd)?;

    project_root.read_tasks_file()
}

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
    pub fn find_project_root_starting_from(cwd: &'a Path) -> JuteResult<Self> {
        cwd.ancestors()
            .find(|dir| dir.join(".jute").is_dir())
            .map(|root| ProjectRoot { root })
            .ok_or_else(|| JuteError::NoProjectRoot {
                cwd: cwd.to_path_buf(),
            })
    }

    fn jute_dir(&self) -> PathBuf {
        self.root.join(".jute")
    }

    pub fn path(&self) -> &Path {
        self.root
    }

    pub fn read_tasks_file(&self) -> JuteResult<String> {
        let path = self.jute_dir().join("tasks.jute");

        read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
    }
}
