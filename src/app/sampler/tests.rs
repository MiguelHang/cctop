use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use super::{Running, Sampler, Update, spawn};
use crate::app::context::tests::Counting;
use crate::app::context::{Found, PaneContext};
use crate::app::tests::{STATUS, fake_sampler, first_sample};
use crate::cornercase::Client;
use crate::cornercase::fake::FakeRunner;
use crate::procs::Sample;
use crate::procs::fake::FakeSource;
use crate::test_util::wait_until;

const FAST: Duration = Duration::from_millis(10);

fn failing_sampler() -> Sampler<FakeRunner, FakeSource> {
    let failing = FakeRunner::default().fail(&["status", "--json"], "server not running");
    Sampler::new(Client::new(failing), FakeSource::new([first_sample(Instant::now())]))
}

fn with_claudes() -> Sample {
    let mut sample = first_sample(Instant::now());
    for process in sample.processes.iter_mut().filter(|process| [101, 300].contains(&process.pid)) {
        process.name = "claude".to_owned();
    }
    sample
}

fn claude_sampler() -> Sampler<FakeRunner, FakeSource, Counting> {
    let runner = FakeRunner::default().answer(&["status", "--json"], STATUS);
    Sampler::new(Client::new(runner), FakeSource::new([with_claudes()]))
        .with_contexts(Counting::default(), Some("/home/test".into()))
}

fn context_of(update: Option<Update>) -> Option<PaneContext> {
    update.expect("an update").expect("a reading").context
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

mod selecting {
    use super::*;

    #[test]
    fn reads_nothing_until_a_pane_is_selected() {
        let mut sampler = claude_sampler();
        assert_eq!(sampler.tick().expect("a reading").context, None);
        assert_eq!(sampler.contexts.asked, [Vec::<u32>::new()]);
    }

    #[test]
    fn changing_pane_reads_the_session_of_the_new_one_only() {
        let mut sampler = claude_sampler();
        sampler.tick().expect("a reading");
        let first = context_of(sampler.select(Some(1)));
        assert_eq!(first, Some(PaneContext { pane: 1, found: Found::NoSession(101) }));
        let second = context_of(sampler.select(Some(7)));
        assert_eq!(second, Some(PaneContext { pane: 7, found: Found::NoSession(300) }));
        assert_eq!(sampler.contexts.asked, [vec![], vec![101], vec![300]]);
        sampler.tick().expect("a reading");
        assert_eq!(sampler.contexts.asked.last(), Some(&vec![300]));
    }

    #[test]
    fn before_the_first_reading_waits_for_the_tick() {
        let mut sampler = claude_sampler();
        assert!(sampler.select(Some(1)).is_none());
        let context = sampler.tick().expect("a reading").context;
        assert_eq!(context.map(|context| context.pane), Some(1));
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
    fn a_selected_pane_is_answered_without_waiting_for_the_interval() {
        let running = spawn(claude_sampler(), Duration::from_secs(3600));
        receive(&running, 1);
        running.panes.send(Some(1)).expect("the sampler listens");
        let updates = receive(&running, 1);
        let context = updates[0].as_ref().expect("a reading").context.as_ref().map(|context| context.pane);
        assert_eq!(context, Some(1));
    }

    #[test]
    fn keeps_retrying_while_cornercase_fails() {
        let running = spawn(failing_sampler(), FAST);
        let updates = receive(&running, 2);
        assert!(updates.iter().all(Result::is_err));
    }
}
