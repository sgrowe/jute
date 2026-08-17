use crate::command_runner::CommandRunner;
use crate::error::{Context, JuteError, JuteResult};
use std::{
    env::{
        consts::{ARCH, OS},
        current_exe,
    },
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
};

const RUN_SCRIPT_TEMPLATE: &str = include_str!("../templates/run");
const DEFAULT_TASKS_FILE: &str = include_str!("../../examples/default.jute");

const EXECUTABLE_MODE: u32 = 0o755;

const SUPPORTED_PLATFORMS: [(&str, &str); 3] = [
    ("macos", "aarch64"),
    ("linux", "x86_64"),
    ("linux", "aarch64"),
];

pub fn install<R: CommandRunner>(cwd: &Path, runner: &mut R) -> JuteResult<()> {
    let jute_dir = cwd.join(".jute");
    fs::create_dir_all(&jute_dir)
        .with_context(|| format!("failed to create {}", jute_dir.display()))?;

    write_run_script(&jute_dir)?;
    write_default_tasks_file_if_absent(&jute_dir)?;

    let bin_dir = jute_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;

    install_current_binary(&bin_dir)?;
    download_other_platform_binaries(&bin_dir, runner)?;

    println!("Installed jute into {}", jute_dir.display());

    Ok(())
}

fn write_run_script(jute_dir: &Path) -> JuteResult<()> {
    let run_path = jute_dir.join("run");

    fs::write(&run_path, RUN_SCRIPT_TEMPLATE)
        .with_context(|| format!("failed to write {}", run_path.display()))?;

    set_executable(&run_path)
}

fn write_default_tasks_file_if_absent(jute_dir: &Path) -> JuteResult<()> {
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

fn install_current_binary(bin_dir: &Path) -> JuteResult<()> {
    let src =
        current_exe().context("failed to determine path of the currently running executable")?;

    copy_executable_atomically(&src, bin_dir, &binary_file_name())
}

/// `jute-<os>-<arch>`, e.g. `jute-macos-aarch64`, `jute-linux-x86_64`.
fn binary_file_name() -> String {
    format!("jute-{OS}-{ARCH}")
}

fn download_other_platform_binaries<R: CommandRunner>(
    bin_dir: &Path,
    runner: &mut R,
) -> JuteResult<()> {
    for (os, arch) in SUPPORTED_PLATFORMS {
        let file_name = format!("jute-{os}-{arch}");

        if file_name != binary_file_name() {
            download_binary(bin_dir, &file_name, runner)?;
        }
    }

    Ok(())
}

fn download_binary<R: CommandRunner>(
    bin_dir: &Path,
    file_name: &str,
    runner: &mut R,
) -> JuteResult<()> {
    println!("Downloading {file_name}...");

    let url = release_url(file_name);

    place_executable_atomically(bin_dir, file_name, |tmp_path| {
        let mut command = Command::new("curl");
        command
            .args([
                "--fail",
                "--location",
                // The progress meter would drown out the actual error when
                // captured stderr is replayed on failure.
                "--no-progress-meter",
                "--retry",
                "5",
                "--max-time",
                "300",
            ])
            .arg("--output")
            .arg(tmp_path)
            .arg(&url);

        let output = runner
            .output(command)
            .with_context(|| format!("failed to run curl to download {url}"))?;

        if !output.status.success() {
            return Err(JuteError::DownloadFailed {
                url: url.clone(),
                code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(())
    })
}

fn release_url(file_name: &str) -> String {
    format!(
        "https://github.com/sgrowe/jute/releases/download/v{}/{file_name}",
        env!("CARGO_PKG_VERSION")
    )
}

/// `fs::copy` truncates its destination first, so copying `src` onto itself
/// — e.g. re-running `self.install` via the already-installed binary — would
/// corrupt it; producing a temp file and renaming into place avoids that.
fn copy_executable_atomically(src: &Path, bin_dir: &Path, file_name: &str) -> JuteResult<()> {
    place_executable_atomically(bin_dir, file_name, |tmp_path| {
        fs::copy(src, tmp_path).with_context(|| {
            format!("failed to copy {} to {}", src.display(), tmp_path.display())
        })?;

        Ok(())
    })
}

/// Has `produce` create the file at a same-directory temp path, then marks it
/// executable and renames it into place. `rename` is atomic and safe even if
/// the destination is currently executing; on any failure the temp file is
/// removed, however far we got.
fn place_executable_atomically(
    bin_dir: &Path,
    file_name: &str,
    produce: impl FnOnce(&Path) -> JuteResult<()>,
) -> JuteResult<()> {
    let dest = bin_dir.join(file_name);
    let tmp_path = bin_dir.join(format!(".{file_name}.tmp.{}", std::process::id()));

    let result = (|| -> JuteResult<()> {
        produce(&tmp_path)?;
        set_executable(&tmp_path)?;
        fs::rename(&tmp_path, &dest).with_context(|| {
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

fn set_executable(path: &Path) -> JuteResult<()> {
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
    use crate::command_runner::test_utils::{
        RecordingRunner, output_killed_by_signal, output_with_exit_code,
    };
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

    /// The spec's platform names verbatim, minus the host's — independent of
    /// the implementation's platform list.
    fn remaining_platform_names() -> Vec<&'static str> {
        [
            "jute-macos-aarch64",
            "jute-linux-x86_64",
            "jute-linux-aarch64",
        ]
        .into_iter()
        .filter(|name| *name != binary_file_name())
        .collect()
    }

    fn expected_curl_argv(root: &Path, name: &str) -> Vec<String> {
        let tmp_path = root
            .join(".jute/bin")
            .join(format!(".{name}.tmp.{}", std::process::id()));

        vec![
            "curl".to_string(),
            "--fail".to_string(),
            "--location".to_string(),
            "--no-progress-meter".to_string(),
            "--retry".to_string(),
            "5".to_string(),
            "--max-time".to_string(),
            "300".to_string(),
            "--output".to_string(),
            tmp_path.to_string_lossy().into_owned(),
            format!(
                "https://github.com/sgrowe/jute/releases/download/v{}/{name}",
                env!("CARGO_PKG_VERSION")
            ),
        ]
    }

    fn recorded_argvs(runner: &RecordingRunner) -> Vec<Vec<String>> {
        runner
            .commands
            .iter()
            .map(|c| {
                c.command
                    .iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn downloads_binaries_for_the_other_supported_platforms_via_curl() {
        let root = fresh_project_root("jute-install-downloads-other-platform-binaries");
        let mut runner = RecordingRunner::default();

        install(&root, &mut runner).unwrap();

        let expected: Vec<Vec<String>> = remaining_platform_names()
            .iter()
            .map(|name| expected_curl_argv(&root, name))
            .collect();
        assert_eq!(recorded_argvs(&runner), expected);
    }

    #[test]
    fn downloaded_binaries_are_executable() {
        let root = fresh_project_root("jute-install-downloaded-binaries-are-executable");

        install(&root, &mut RecordingRunner::default()).unwrap();

        let modes: Vec<u32> = remaining_platform_names()
            .iter()
            .map(|name| {
                fs::metadata(root.join(".jute/bin").join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777
            })
            .collect();
        assert_eq!(modes, vec![0o755; remaining_platform_names().len()]);
    }

    #[test]
    fn a_failed_download_fails_the_install_and_stops_at_the_first_failure() {
        let root = fresh_project_root("jute-install-fails-on-first-failed-download");
        let mut runner = RecordingRunner::default();
        runner.queued_outputs.push_back(output_with_exit_code(
            22,
            "",
            "curl: (22) The requested URL returned error: 404\n",
        ));

        let error = install(&root, &mut runner).unwrap_err();

        let failed = remaining_platform_names()[0];
        assert_eq!(
            error.to_string(),
            format!(
                "failed to download https://github.com/sgrowe/jute/releases/download/v{}/{failed}\n\
                 --- curl stdout ---\n\
                 --- curl stderr ---\n\
                 curl: (22) The requested URL returned error: 404\n\
                 curl failed with exit code 22",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert_eq!(error.exit_code(), 1);
        assert_eq!(
            recorded_argvs(&runner),
            vec![expected_curl_argv(&root, failed)]
        );
    }

    #[test]
    fn a_signal_killed_curl_reports_the_signal_instead_of_an_exit_code() {
        let root = fresh_project_root("jute-install-signal-killed-curl");
        let mut runner = RecordingRunner::default();
        runner.queued_outputs.push_back(output_killed_by_signal(9));

        let error = install(&root, &mut runner).unwrap_err();

        let failed = remaining_platform_names()[0];
        assert_eq!(
            error.to_string(),
            format!(
                "failed to download https://github.com/sgrowe/jute/releases/download/v{}/{failed}\n\
                 --- curl stdout ---\n\
                 --- curl stderr ---\n\
                 curl was terminated by a signal",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert_eq!(error.exit_code(), 1);
    }

    #[test]
    fn a_failed_download_leaves_no_partial_file_behind() {
        let root = fresh_project_root("jute-install-failed-download-leaves-no-partial-file");
        let mut runner = RecordingRunner::default();
        runner
            .queued_outputs
            .push_back(output_with_exit_code(22, "", ""));

        install(&root, &mut runner).unwrap_err();

        let mut expected = vec![
            format!(".jute/bin/{}", binary_file_name()),
            ".jute/run".to_string(),
            ".jute/tasks.jute".to_string(),
        ];
        expected.sort();
        assert_eq!(relative_file_paths(&root, &root.join(".jute")), expected);
    }

    #[test]
    fn fresh_install_creates_the_full_dot_jute_structure() {
        let root = fresh_project_root("jute-install-creates-full-structure");

        install(&root, &mut RecordingRunner::default()).unwrap();

        let expected_bin = format!(".jute/bin/{}", binary_file_name());
        let mut expected = vec![
            expected_bin,
            ".jute/run".to_string(),
            ".jute/tasks.jute".to_string(),
        ];
        expected.extend(
            remaining_platform_names()
                .iter()
                .map(|name| format!(".jute/bin/{name}")),
        );
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
        install(&root, &mut RecordingRunner::default()).unwrap();

        let tasks_path = root.join(".jute/tasks.jute");
        fs::write(&tasks_path, "custom:\n  echo custom\n").unwrap();

        install(&root, &mut RecordingRunner::default()).unwrap();

        assert_eq!(
            fs::read_to_string(&tasks_path).unwrap(),
            "custom:\n  echo custom\n"
        );
    }

    #[test]
    fn reinstalling_overwrites_run_and_the_binary() {
        let root = fresh_project_root("jute-install-overwrites-run-and-binary");
        install(&root, &mut RecordingRunner::default()).unwrap();

        let run_path = root.join(".jute/run");
        let bin_path = root.join(".jute/bin").join(binary_file_name());
        fs::write(&run_path, "corrupted").unwrap();
        fs::write(&bin_path, "corrupted").unwrap();

        install(&root, &mut RecordingRunner::default()).unwrap();

        assert_eq!(fs::read_to_string(&run_path).unwrap(), RUN_SCRIPT_TEMPLATE);
        assert_eq!(
            fs::read(&bin_path).unwrap(),
            fs::read(current_exe().unwrap()).unwrap()
        );
    }

    #[test]
    fn run_script_and_binary_are_executable() {
        let root = fresh_project_root("jute-install-sets-executable-bit");
        install(&root, &mut RecordingRunner::default()).unwrap();

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
        install(&root, &mut RecordingRunner::default()).unwrap();

        let bin_path = root.join(".jute/bin").join(binary_file_name());

        assert_eq!(
            fs::read(&bin_path).unwrap(),
            fs::read(current_exe().unwrap()).unwrap()
        );
    }

    #[test]
    fn run_script_finds_and_execs_the_installed_binary() {
        let root = fresh_project_root("jute-install-run-script-e2e");
        install(&root, &mut RecordingRunner::default()).unwrap();

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
