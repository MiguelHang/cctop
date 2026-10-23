use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use rstest::rstest;

use super::super::fake::GB;
use super::super::rules::Thresholds;
use super::{Clock, Watch};
use crate::cornercase::Client;
use crate::cornercase::fake::FakeRunner;
use crate::procs::fake::FakeSource;
use crate::procs::{Process, Sample};

const BOOT: &str = "4242-1700000000000000000";
const STATUS: &[&str] = &["status", "--json"];
const INTERVAL: Duration = Duration::from_secs(5);
const MEMORY_ALERT: &str = "cctop: feat/x › claude 5.0 GB — node server.js 5.0 GB";
const THRESHOLDS: Thresholds = Thresholds { memory: 4 * GB, cpu: 300.0, cpu_sustain: Duration::ZERO };

struct FakeClock {
    now: Cell<Instant>,
    sleeps: RefCell<Vec<Duration>>,
}

impl FakeClock {
    fn new(start: Instant) -> Self {
        Self { now: Cell::new(start), sleeps: RefCell::new(Vec::new()) }
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.now.get()
    }

    fn sleep(&self, duration: Duration) {
        self.sleeps.borrow_mut().push(duration);
        let now = self.now.get();
        self.now.set(now.checked_add(duration).unwrap_or(now));
    }
}

fn status_json() -> String {
    format!(
        r#"{{"boot": "{BOOT}", "projects": [{{"id": 1, "workspaces": [{{"id": 3, "name": "feat/x", "tabs": [
            {{"id": 4, "panes": [{{"id": 1, "agent": "claude"}}]}}]}}]}}]}}"#
    )
}

fn node(memory: u64, cpu_seconds: u64) -> Process {
    Process {
        pid: 10,
        name: "node".to_owned(),
        args: vec!["node".to_owned(), "server.js".to_owned()],
        memory: Some(memory),
        cpu_time: Duration::from_secs(cpu_seconds),
        pane: Some(1),
        server: Some(BOOT.to_owned()),
        ..Process::default()
    }
}

fn samples(start: Instant, processes: impl IntoIterator<Item = Process>) -> FakeSource {
    let every = (0..).map(|tick| start + INTERVAL * tick);
    FakeSource::new(every.zip(processes).map(|(taken, process)| Sample { taken, processes: vec![process] }))
}

fn times(mut left: usize) -> impl FnMut() -> bool {
    move || {
        let more = left > 0;
        left = left.saturating_sub(1);
        more
    }
}

fn notifies(runner: &FakeRunner) -> Vec<String> {
    let calls = runner.calls().into_iter().filter(|argv| argv[0] == "notify");
    calls
        .map(|argv| {
            assert_eq!(argv[1], "--", "the text is passed after --");
            argv[2].clone()
        })
        .collect()
}

fn run(runner: &FakeRunner, source: FakeSource, ticks: usize) -> (Vec<u8>, Vec<Duration>) {
    let clock = FakeClock::new(Instant::now());
    let mut watch = Watch::new(Client::new(runner), source, Vec::new(), THRESHOLDS);
    watch.run(&clock, INTERVAL, times(ticks));
    (watch.log, clock.sleeps.into_inner())
}

mod alerting {
    use super::*;

    #[test]
    fn notifies_once_when_a_pane_crosses_the_memory_threshold() {
        let runner = FakeRunner::default().answer(STATUS, &status_json()).answer(&["notify", "--", MEMORY_ALERT], "");
        let start = Instant::now();
        let source = samples(start, [node(GB, 0), node(5 * GB, 0), node(5 * GB, 0)]);
        let (log, _) = run(&runner, source, 3);
        assert_eq!(notifies(&runner), [MEMORY_ALERT]);
        assert_eq!(String::from_utf8_lossy(&log), "");
    }

    #[test]
    fn measures_cpu_between_two_samples() {
        let runner = FakeRunner::default().answer(STATUS, &status_json());
        let start = Instant::now();
        let source = samples(start, [node(GB, 0), node(GB, 20)]);
        run(&runner, source, 2);
        assert_eq!(notifies(&runner), ["cctop: feat/x › claude 400% — node server.js 400%"]);
    }
}

mod own_actions {
    use super::*;

    fn popup(action_of: &str) -> Process {
        Process { pid: 11, action_of: Some(action_of.to_owned()), ..node(5 * GB, 0) }
    }

    fn with_popup(start: Instant, action_of: &str) -> FakeSource {
        let every = (0..).map(|tick| start + INTERVAL * tick);
        let processes = |memory| vec![node(memory, 0), popup(action_of)];
        FakeSource::new(every.zip([GB, GB]).map(|(taken, memory)| Sample { taken, processes: processes(memory) }))
    }

    #[test]
    fn a_cctop_launched_by_its_own_action_does_not_count_in_the_pane() {
        let runner = FakeRunner::default().answer(STATUS, &status_json());
        run(&runner, with_popup(Instant::now(), "cctop"), 2);
        assert_eq!(notifies(&runner), Vec::<String>::new());
    }

    #[test]
    fn an_action_of_another_plugin_still_counts() {
        let alert = "cctop: feat/x › claude 6.0 GB — node server.js 5.0 GB";
        let runner = FakeRunner::default().answer(STATUS, &status_json()).answer(&["notify", "--", alert], "");
        run(&runner, with_popup(Instant::now(), "other"), 2);
        assert_eq!(notifies(&runner), [alert]);
    }
}

mod failures {
    use super::*;

    #[test]
    fn keeps_going_when_status_fails() {
        let runner = FakeRunner::default().fail(STATUS, "no server");
        let source = samples(Instant::now(), [node(5 * GB, 0)]);
        let (log, sleeps) = run(&runner, source, 3);
        assert_eq!(runner.calls().len(), 3);
        assert_eq!(sleeps, [INTERVAL; 3]);
        let line = "cctop watch: `status --json` failed: no server\n";
        assert_eq!(String::from_utf8_lossy(&log), line.repeat(3));
    }

    #[test]
    fn logs_a_failed_notify_and_counts_it_as_sent() {
        let runner = FakeRunner::default().answer(STATUS, &status_json());
        let source = samples(Instant::now(), [node(5 * GB, 0)]);
        let (log, _) = run(&runner, source, 2);
        assert_eq!(notifies(&runner), [MEMORY_ALERT]);
        let line = format!("cctop watch: `notify -- {MEMORY_ALERT}` failed: no answer for this argv\n");
        assert_eq!(String::from_utf8_lossy(&log), line);
    }

    fn one_round_each(runners: &[&FakeRunner]) {
        let clock = FakeClock::new(Instant::now());
        let source = samples(Instant::now(), [node(5 * GB, 0)]);
        let mut watch = Watch::new(Client::new(runners[0]), source, Vec::new(), THRESHOLDS);
        for runner in runners {
            watch = Watch { client: Client::new(*runner), ..watch };
            watch.run(&clock, INTERVAL, times(1));
        }
    }

    #[test]
    fn does_not_alert_again_when_status_comes_back() {
        let working = FakeRunner::default().answer(STATUS, &status_json());
        let failing = FakeRunner::default().fail(STATUS, "no server");
        one_round_each(&[&working, &failing, &working]);
        assert_eq!(notifies(&working), [MEMORY_ALERT]);
    }

    #[test]
    fn alerts_after_status_comes_back() {
        let failing = FakeRunner::default().fail(STATUS, "no server");
        let working = FakeRunner::default().answer(STATUS, &status_json());
        one_round_each(&[&failing, &working]);
        assert_eq!(notifies(&working), [MEMORY_ALERT]);
    }
}

mod pacing {
    use super::*;

    struct SlowClock {
        clock: FakeClock,
        step: Duration,
    }

    impl Clock for SlowClock {
        fn now(&self) -> Instant {
            let now = self.clock.now();
            self.clock.now.set(now + self.step);
            now
        }

        fn sleep(&self, duration: Duration) {
            self.clock.sleep(duration);
        }
    }

    #[test]
    fn sleeps_the_interval_between_ticks() {
        let runner = FakeRunner::default().answer(STATUS, &status_json());
        let (_, sleeps) = run(&runner, samples(Instant::now(), [node(GB, 0)]), 3);
        assert_eq!(sleeps, [INTERVAL; 3]);
    }

    #[rstest]
    #[case::a_quick_tick(Duration::from_secs(2), Duration::from_secs(3))]
    #[case::a_tick_longer_than_the_interval(Duration::from_secs(9), Duration::ZERO)]
    fn counts_the_time_a_tick_takes_without_bursting_to_catch_up(#[case] step: Duration, #[case] sleep: Duration) {
        let runner = FakeRunner::default().answer(STATUS, &status_json());
        let clock = SlowClock { clock: FakeClock::new(Instant::now()), step };
        let source = samples(Instant::now(), [node(GB, 0)]);
        let mut watch = Watch::new(Client::new(&runner), source, Vec::new(), THRESHOLDS);
        watch.run(&clock, INTERVAL, times(3));
        assert_eq!(clock.clock.sleeps.into_inner(), [sleep; 3]);
    }

    #[test]
    fn sleeps_an_interval_too_long_for_the_clock_without_panicking() {
        let runner = FakeRunner::default().answer(STATUS, &status_json());
        let clock = FakeClock::new(Instant::now());
        let mut watch =
            Watch::new(Client::new(&runner), samples(Instant::now(), [node(GB, 0)]), Vec::new(), THRESHOLDS);
        watch.run(&clock, Duration::MAX, times(2));
        assert_eq!(clock.sleeps.into_inner(), [Duration::MAX; 2]);
    }
}
