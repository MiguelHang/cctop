use std::collections::HashMap;
use std::sync::Mutex;

use super::Runner;
use crate::error::{Error, Result};

#[derive(Default)]
pub(crate) struct FakeRunner {
    answers: HashMap<Vec<String>, std::result::Result<String, String>>,
    calls: Mutex<Vec<Vec<String>>>,
}

impl FakeRunner {
    pub(crate) fn answer(mut self, argv: &[&str], stdout: &str) -> Self {
        self.answers.insert(owned(argv), Ok(stdout.to_owned()));
        self
    }

    pub(crate) fn fail(mut self, argv: &[&str], stderr: &str) -> Self {
        self.answers.insert(owned(argv), Err(stderr.to_owned()));
        self
    }

    pub(crate) fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().expect("calls lock").clone()
    }
}

impl Runner for FakeRunner {
    fn run(&self, argv: &[String]) -> Result<String> {
        self.calls.lock().expect("calls lock").push(argv.to_vec());
        match self.answers.get(argv) {
            Some(Ok(stdout)) => Ok(stdout.clone()),
            Some(Err(stderr)) => Err(Error::Command { argv: argv.to_vec(), stderr: stderr.clone() }),
            None => Err(Error::Command { argv: argv.to_vec(), stderr: "no answer for this argv".to_owned() }),
        }
    }
}

fn owned(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| (*word).to_owned()).collect()
}
