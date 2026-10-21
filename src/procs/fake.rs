use std::cell::RefCell;
use std::collections::VecDeque;

use super::{ProcessSource, Sample};
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
