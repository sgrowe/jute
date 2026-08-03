use anyhow::{Context, anyhow};
use std::{
    env::{
        consts::{ARCH, OS},
        current_exe,
    },
    fs::{self, OpenOptions},
    io::{ErrorKind, Write as _},
    os::unix::fs::PermissionsExt,
    path::Path,
};

const RUN_SCRIPT_TEMPLATE: &str = include_str!("../templates/run");
const DEFAULT_TASKS_FILE: &str = include_str!("../../examples/default.jute");

const EXECUTABLE_MODE: u32 = 0o755;

pub fn install(cwd: &Path) -> anyhow::Result<()> {
    let jute_dir = cwd.join(".jute");
    fs::create_dir_all(&jute_dir)
        .with_context(|| format!("failed to create {}", jute_dir.display()))?;

    write_run_script(&jute_dir)?;
    write_default_tasks_file_if_absent(&jute_dir)?;

    let bin_dir = jute_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;

    install_current_binary(&bin_dir)?;

    println!("Installed jute into {}", jute_dir.display());

    Ok(())
}

fn write_run_script(jute_dir: &Path) -> anyhow::Result<()> {
    let run_path = jute_dir.join("run");

    fs::write(&run_path, RUN_SCRIPT_TEMPLATE)
        .with_context(|| format!("failed to write {}", run_path.display()))?;

    set_executable(&run_path)
}

fn write_default_tasks_file_if_absent(jute_dir: &Path) -> anyhow::Result<()> {
    let tasks_path = jute_dir.join("tasks.jute");

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tasks_path)
    {
        Ok(mut file) => file
            .write_all(DEFAULT_TASKS_FILE.as_bytes())
            .with_context(|| format!("failed to write {}", tasks_path.display())),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e).with_context(|| format!("failed to create {}", tasks_path.display())),
    }
}

fn install_current_binary(bin_dir: &Path) -> anyhow::Result<()> {
    let src =
        current_exe().context("failed to determine path of the currently running executable")?;
    let dest = bin_dir.join(binary_file_name());

    copy_executable_atomically(&src, &dest)
}

/// `jute-<os>-<arch>`, e.g. `jute-macos-aarch64`, `jute-linux-x86_64`.
fn binary_file_name() -> String {
    format!("jute-{OS}-{ARCH}")
}

/// `fs::copy` truncates its destination first, so copying `src` onto itself
/// — e.g. re-running `--install` via the already-installed binary — would
/// corrupt it. Copying into a same-directory temp file and renaming into
/// place avoids that; `rename` is atomic and safe even if `dest` is
/// currently executing.
fn copy_executable_atomically(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let bin_dir = dest
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", dest.display()))?;
    let file_name = dest
        .file_name()
        .ok_or_else(|| anyhow!("{} has no file name", dest.display()))?
        .to_string_lossy();
    let tmp_path = bin_dir.join(format!(".{file_name}.tmp.{}", std::process::id()));

    // A closure rather than a block, so that `?` returns to here and the
    // temp file below is cleaned up however far we got.
    let result = (|| -> JuteResult<()> {
        fs::copy(src, &tmp_path).with_context(|| {
            format!("failed to copy {} to {}", src.display(), tmp_path.display())
        })?;
        set_executable(&tmp_path)?;
        fs::rename(&tmp_path, dest).with_context(|| {
            format!(
                "failed to move {} into place at {}",
                tmp_path.display(),
                dest.display()
            )
        })
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }

    result
}

fn set_executable(path: &Path) -> anyhow::Result<()> {
    let mut perms = fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?
        .permissions();
    perms.set_mode(EXECUTABLE_MODE);
    fs::set_permissions(path, perms)
        .with_context(|| format!("failed to set permissions on {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;
    use std::process::Command;

    /// Guaranteed-empty, since tests assert the complete set of paths
    /// `install` creates and stale files from a previous run would corrupt that.
    fn fresh_project_root(name: &str) -> PathBuf {
        let root = env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        root
    }

    fn relative_file_paths(root: &Path, dir: &Path) -> Vec<String> {
        let mut paths = Vec::new();

        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();

            if path.is_dir() {
                paths.extend(relative_file_paths(root, &path));
            } else {
                paths.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }

        paths.sort();

        paths
    }

    #[test]
    fn fresh_install_creates_the_full_dot_jute_structure() {
        let root = fresh_project_root("jute-install-creates-full-structure");

        install(&root).unwrap();

        let expected_bin = format!(".jute/bin/{}", binary_file_name());
        let mut expected = vec![
            expected_bin,
            ".jute/run".to_string(),
            ".jute/tasks.jute".to_string(),
        ];
        expected.sort();

        assert_eq!(relative_file_paths(&root, &root.join(".jute")), expected);
        assert_eq!(
            fs::read_to_string(root.join(".jute/tasks.jute")).unwrap(),
            DEFAULT_TASKS_FILE
        );
    }

    #[test]
    fn reinstalling_does_not_overwrite_an_existing_tasks_file() {
        let root = fresh_project_root("jute-install-preserves-custom-tasks-file");
        install(&root).unwrap();

        let tasks_path = root.join(".jute/tasks.jute");
        fs::write(&tasks_path, "custom:\n  echo custom\n").unwrap();

        install(&root).unwrap();

        assert_eq!(
            fs::read_to_string(&tasks_path).unwrap(),
            "custom:\n  echo custom\n"
        );
    }

    #[test]
    fn reinstalling_overwrites_run_and_the_binary() {
        let root = fresh_project_root("jute-install-overwrites-run-and-binary");
        install(&root).unwrap();

        let run_path = root.join(".jute/run");
        let bin_path = root.join(".jute/bin").join(binary_file_name());
        fs::write(&run_path, "corrupted").unwrap();
        fs::write(&bin_path, "corrupted").unwrap();

        install(&root).unwrap();

        assert_eq!(fs::read_to_string(&run_path).unwrap(), RUN_SCRIPT_TEMPLATE);
        assert_eq!(
            fs::read(&bin_path).unwrap(),
            fs::read(current_exe().unwrap()).unwrap()
        );
    }

    #[test]
    fn run_script_and_binary_are_executable() {
        let root = fresh_project_root("jute-install-sets-executable-bit");
        install(&root).unwrap();

        let run_mode = fs::metadata(root.join(".jute/run"))
            .unwrap()
            .permissions()
            .mode();
        let bin_mode = fs::metadata(root.join(".jute/bin").join(binary_file_name()))
            .unwrap()
            .permissions()
            .mode();

        assert_eq!(run_mode & 0o777, 0o755);
        assert_eq!(bin_mode & 0o777, 0o755);
    }

    #[test]
    fn binary_is_copied_byte_for_byte() {
        let root = fresh_project_root("jute-install-copies-binary-verbatim");
        install(&root).unwrap();

        let bin_path = root.join(".jute/bin").join(binary_file_name());

        assert_eq!(
            fs::read(&bin_path).unwrap(),
            fs::read(current_exe().unwrap()).unwrap()
        );
    }

    #[test]
    fn run_script_finds_and_execs_the_installed_binary() {
        let root = fresh_project_root("jute-install-run-script-e2e");
        install(&root).unwrap();

        // `current_exe()` under `cargo test` is the test binary, not jute's
        // CLI, so this only checks the run script's own mechanics (self-
        // location, binary lookup, exec) — not task dispatch. The filter arg
        // must match no test name, or this would recursively re-run the
        // whole suite.
        let status = Command::new("sh")
            .arg(root.join(".jute/run"))
            .arg("this_should_match_no_test_zzz")
            .current_dir(&root)
            .status()
            .unwrap();

        assert!(status.success());
    }
}
