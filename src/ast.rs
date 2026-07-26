use std::{borrow::Cow, path::PathBuf};

#[derive(Debug, PartialEq, Default)]
pub struct TaskFile<'a> {
    pub tasks: Vec<Task<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct Task<'a> {
    pub name: &'a str,
    pub steps: Vec<Step<'a>>,
}

impl<'a> Task<'a> {
    pub fn new(name: &'a str, steps: Vec<Step<'a>>) -> Self {
        Self { name, steps }
    }
}

#[derive(Debug, PartialEq)]
pub enum Step<'a> {
    Command(Cow<'a, str>),
    With {
        env: Vec<EnvVar<'a>>,
        steps: Vec<Step<'a>>,
    },
    InSubDir {
        path: PathBuf,
        steps: Vec<Step<'a>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct EnvVar<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a> EnvVar<'a> {
    pub fn new(name: &'a str, value: &'a str) -> Self {
        Self { name, value }
    }
}
