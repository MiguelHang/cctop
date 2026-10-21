use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use super::{Running, Sampler, Update, spawn};
use crate::app::tests::{fake_sampler, first_sample};
use crate::cornercase::Client;
use crate::cornercase::fake::FakeRunner;
use crate::procs::fake::FakeSource;
use crate::test_util::wait_until;

const FAST: Duration = Duration::from_millis(10);

fn failing_sampler() -> Sampler<FakeRunner, FakeSource> {
    let failing = FakeRunner::default().fail(&["status", "--json"], "server not running");
    Sampler::new(Client::new(failing), FakeSource::new([first_sample(Instant::now())]))
}

fn receive(running: &Running, count: usize) -> Vec<Update> {
    let mut updates = Vec::new();
    wait_until("the sampler sends its updates", || {
        match running.updates.try_recv() {
            Ok(update) => updates.push(update),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("the sampler stopped"),
        }
        updates.len() >= count
    });
    updates
}

mod tick {
    use super::*;

    #[test]
    fn the_first_reading_has_no_cpu_and_the_second_has() {
        let mut sampler = fake_sampler();
        assert!(sampler.tick().expect("first reading").cpu.is_empty());
        assert!(!sampler.tick().expect("second reading").cpu.is_empty());
    }

    #[test]
    fn a_failed_status_names_cornercase() {
        let error = failing_sampler().tick().expect_err("status fails");
        assert!(error.starts_with("cornercase: "), "{error}");
        assert!(error.contains("server not running"), "{error}");
    }
}

mod thread {
    use super::*;

    #[test]
    fn sends_a_reading_every_interval() {
        let running = spawn(fake_sampler(), FAST);
        let updates = receive(&running, 2);
        assert!(updates.iter().all(Result::is_ok));
    }

    #[test]
    fn keeps_retrying_while_cornercase_fails() {
        let running = spawn(failing_sampler(), FAST);
        let updates = receive(&running, 2);
        assert!(updates.iter().all(Result::is_err));
    }
}
