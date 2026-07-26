use crate::cli_args::CliArgs;

mod cli_args;

fn main() {
    // parse CLI args
    let args: CliArgs = argh::from_env();

    dbg!(&args);

    // find nearest .jute folder
    // read .jute/tasks.jute
    // parse tasks.jute

    // exec tasks from CLI args
}
