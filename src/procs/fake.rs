use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::kill::{Killer, Target};
use super::{Pid, ProcessSource, Sample};
use crate::error::Result;

pub(crate) struct FakeSource {
    samples: RefCell<VecDeque<Sample>>,
}

impl FakeSource {
    pub(crate) fn new(samples: impl IntoIterator<Item = Sample>) -> Self {
        Self { samples: RefCell::new(samples.into_iter().collect()) }
    }
}

impl ProcessSource for FakeSource {
    fn sample(&self) -> Result<Sample> {
        let mut samples = self.samples.borrow_mut();
        let next = if samples.len() > 1 { samples.pop_front() } else { samples.front().cloned() };
        Ok(next.expect("the fake source has at least one sample"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sent {
    Term(Pid),
    TermGroup(Pid),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FakeKiller {
    sent: Rc<RefCell<Vec<Sent>>>,
}

impl FakeKiller {
    pub(crate) fn sent(&self) -> Vec<Sent> {
        self.sent.borrow().clone()
    }
}

impl Killer for FakeKiller {
    fn term(&self, target: Target) -> Result<()> {
        self.sent.borrow_mut().push(Sent::Term(target.pid));
        Ok(())
    }

    fn term_group(&self, target: Target) -> Result<()> {
        self.sent.borrow_mut().push(Sent::TermGroup(target.pgid));
        Ok(())
    }
}
