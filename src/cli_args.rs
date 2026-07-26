use argh::FromArgs;

#[derive(FromArgs, Debug, PartialEq)]
/// Top level CLI args
pub struct CliArgs {
    #[argh(positional)]
    commands: Vec<String>,
}
