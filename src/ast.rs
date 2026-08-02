use std::{borrow::Cow, collections::BTreeMap, path::PathBuf};

use anyhow::anyhow;

#[derive(Debug, PartialEq, Default)]
pub struct TaskFile<'a> {
    tasks: BTreeMap<&'a str, Task<'a>>,
}

impl<'a> TaskFile<'a> {
    pub fn insert_task(&mut self, t: Task<'a>) -> anyhow::Result<()> {
        match self.tasks.insert(t.name, t) {
            Some(prev) => Err(anyhow!("Duplicate task definitions for {}", prev.name)),
            None => Ok(()),
        }
    }

    pub fn get(&self, name: &str) -> anyhow::Result<&Task<'_>> {
        self.tasks
            .get(name)
            .ok_or_else(|| anyhow!("Task \"{}\" does not exist", name))
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
