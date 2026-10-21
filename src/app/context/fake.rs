use std::collections::HashMap;

use super::Contexts;
use crate::context::Claude;
use crate::context::breakdown::Breakdown;
use crate::procs::Pid;

#[derive(Default)]
pub(in crate::app) struct Counting {
    pub(in crate::app) asked: Vec<Vec<Pid>>,
}

impl Contexts for Counting {
    fn read(&mut self, claudes: &[Claude]) -> HashMap<Pid, Breakdown> {
        self.asked.push(claudes.iter().map(|claude| claude.pid).collect());
        HashMap::new()
    }
}
