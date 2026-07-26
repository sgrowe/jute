use crate::cli_args::CliArgs;
use crate::project_root::find_project_root;
use std::{env, process};

mod cli_args;
mod project_root;

fn main() {
    // parse CLI args
    let args: CliArgs = argh::from_env();

    dbg!(&args);

    // find nearest ancestor containing a .jute folder
    let cwd = env::current_dir().expect("failed to read the current directory");
    let Some(root) = find_project_root(&cwd) else {
        eprintln!("no .jute folder found in {} or any parent", cwd.display());
        process::exit(1);
    };

    dbg!(&root);

    // read .jute/tasks.jute
    // parse tasks.jute

    // exec tasks from CLI args
}
