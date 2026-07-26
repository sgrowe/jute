use std::path::PathBuf;

pub struct TaskFile<'a> {
    tasks: Vec<Task<'a>>,
}

pub struct Task<'a> {
    name: &'a str,
    steps: Vec<Step<'a>>,
}

pub enum Step<'a> {
    Command(&'a str),
    With {
        env: Vec<EnvVar<'a>>,
        steps: Vec<Step<'a>>,
    },
    InSubDir {
        path: PathBuf,
        steps: Vec<Step<'a>>,
    },
}

pub struct EnvVar<'a> {
    name: &'a str,
    value: &'a str,
}
