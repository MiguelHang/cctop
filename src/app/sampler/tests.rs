use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use super::{Running, STATUS_EVERY, Sampler, Update, spawn};
use crate::app::context::fake::Counting;
use crate::app::context::{Found, PaneContext};
use crate::app::tests::{STATUS, fake_sampler, first_sample, process};
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
    match update.expect("an update") {
        Update::Context(context) => context,
        Update::Reading(reading) => panic!("a selection answers with a context, not {reading:?}"),
    }
}

fn is_reading(update: &Update, ok: bool) -> bool {
    matches!(update, Update::Reading(reading) if reading.is_ok() == ok)
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

mod status {
    use super::*;

    fn later(start: Instant, after: Duration, extra_pane: Option<u64>) -> Sample {
        let mut sample = first_sample(start);
        sample.taken = start + after;
        sample.processes.extend(extra_pane.map(|pane| process(900, 10, pane, 5, 0)));
        sample
    }

    fn status_calls(readings: Vec<Sample>, runner: &FakeRunner) -> usize {
        let count = readings.len();
        let mut sampler = Sampler::new(Client::new(runner), FakeSource::new(readings));
        for _ in 0..count {
            let _ = sampler.tick();
        }
        runner.calls().len()
    }

    fn answering() -> FakeRunner {
        FakeRunner::default().answer(&["status", "--json"], STATUS)
    }

    #[test]
    fn is_asked_again_only_after_its_own_interval() {
        let start = Instant::now();
        let samples = [
            Duration::ZERO,
            Duration::from_secs(2),
            STATUS_EVERY.saturating_sub(Duration::from_millis(1)),
            STATUS_EVERY,
        ];
        let samples = samples.into_iter().map(|after| later(start, after, None)).collect();
        assert_eq!(status_calls(samples, &answering()), 2);
    }

    #[test]
    fn reuses_the_snapshot_in_the_readings_between() {
        let start = Instant::now();
        let runner = answering();
        let mut sampler = Sampler::new(Client::new(&runner), FakeSource::new([first_sample(start)]));
        let first = sampler.tick().expect("first reading");
        let second = sampler.tick().expect("second reading");
        assert_eq!(first.snapshot, second.snapshot);
        assert_eq!(runner.calls().len(), 1);
    }

    #[test]
    fn is_asked_at_once_when_a_process_names_an_unknown_pane() {
        let start = Instant::now();
        let samples = vec![later(start, Duration::ZERO, None), later(start, Duration::from_secs(2), Some(99))];
        assert_eq!(status_calls(samples, &answering()), 2);
    }

    #[test]
    fn a_pane_still_unknown_after_asking_is_not_asked_for_again() {
        let start = Instant::now();
        let samples = [Duration::ZERO, Duration::from_secs(2), Duration::from_secs(4)];
        let samples = samples.into_iter().map(|after| later(start, after, Some(99))).collect();
        assert_eq!(status_calls(samples, &answering()), 1);
    }

    #[test]
    fn a_pane_of_another_server_does_not_ask() {
        let start = Instant::now();
        let mut other = later(start, Duration::from_secs(2), Some(99));
        other.processes.last_mut().expect("the extra process").server = Some("another-server".to_owned());
        assert_eq!(status_calls(vec![later(start, Duration::ZERO, None), other], &answering()), 1);
    }

    #[test]
    fn a_failed_status_is_asked_again_on_the_next_tick() {
        let start = Instant::now();
        let failing = FakeRunner::default().fail(&["status", "--json"], "server not running");
        let samples = vec![later(start, Duration::ZERO, None), later(start, Duration::from_secs(2), None)];
        assert_eq!(status_calls(samples, &failing), 2);
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
        assert!(updates.iter().all(|update| is_reading(update, true)));
    }

    #[test]
    fn a_selected_pane_is_answered_without_waiting_for_the_interval() {
        let running = spawn(claude_sampler(), Duration::from_secs(3600));
        receive(&running, 1);
        running.panes.send(Some(1)).expect("the sampler listens");
        let update = receive(&running, 1).pop();
        assert_eq!(context_of(update).map(|context| context.pane), Some(1));
    }

    #[test]
    fn keeps_retrying_while_cornercase_fails() {
        let running = spawn(failing_sampler(), FAST);
        let updates = receive(&running, 2);
        assert!(updates.iter().all(|update| is_reading(update, false)));
    }
}
