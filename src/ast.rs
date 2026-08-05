use std::{borrow::Cow, collections::BTreeMap, path::PathBuf};

use crate::{
    error::{JuteError, JuteResult},
    parser::ParseError,
};

#[derive(Debug, PartialEq, Default)]
pub struct TaskFile<'a> {
    tasks: BTreeMap<&'a str, Task<'a>>,
}

impl<'a> TaskFile<'a> {
    /// A duplicate definition is a fault in the tasks file, so this reports a
    /// [`ParseError`] for the parser to propagate as-is.
    pub fn insert_task(&mut self, t: Task<'a>) -> Result<(), ParseError> {
        match self.tasks.insert(t.name, t) {
            Some(prev) => Err(ParseError::new(format!(
                "Duplicate task definitions for {}",
                prev.name
            ))),
            None => Ok(()),
        }
    }

    pub fn get(&self, name: &str) -> JuteResult<&Task<'_>> {
        self.tasks.get(name).ok_or_else(|| JuteError::TaskNotFound {
            name: name.to_string(),
        })
    }

    pub fn list_tasks(&self) -> impl Iterator<Item = &str> {
        self.tasks.keys().copied()
    }
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
    Command(Cow<'a, str>, Vec<Cow<'a, str>>),
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
