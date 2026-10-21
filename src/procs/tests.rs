use std::time::{Duration, Instant};

use rstest::rstest;

use super::fake::FakeSource;
use super::{Process, ProcessSource, Sample, cpu_percent};

fn process(pid: u32, started: u64, cpu_millis: u64) -> Process {
    Process { pid, started, cpu_time: Duration::from_millis(cpu_millis), ..Process::default() }
}

fn sample(taken: Instant, processes: Vec<Process>) -> Sample {
    Sample { taken, processes }
}

mod cpu {
    use super::*;

    #[rstest]
    #[case::idle(1_000, 1_000, 2_000, Some(0.0))]
    #[case::half_a_core(1_000, 2_000, 2_000, Some(50.0))]
    #[case::one_core(0, 2_000, 2_000, Some(100.0))]
    #[case::two_cores(500, 4_500, 2_000, Some(200.0))]
    #[case::time_went_backwards(2_000, 1_000, 2_000, None)]
    fn is_cpu_time_over_wall_time(
        #[case] before_ms: u64,
        #[case] after_ms: u64,
        #[case] wall_ms: u64,
        #[case] expected: Option<f64>,
    ) {
        let start = Instant::now();
        let prev = sample(start, vec![process(10, 5, before_ms)]);
        let next = sample(start + Duration::from_millis(wall_ms), vec![process(10, 5, after_ms)]);
        let percent = cpu_percent(&prev, &next).get(&10).copied();
        match (percent, expected) {
            (Some(got), Some(want)) => assert!((got - want).abs() < 1e-9, "{got} != {want}"),
            (got, want) => assert_eq!(got, want),
        }
    }

    #[rstest]
    #[case::a_new_pid(process(11, 5, 900))]
    #[case::a_reused_pid(process(10, 6, 900))]
    fn is_none_for(#[case] newcomer: Process) {
        let start = Instant::now();
        let prev = sample(start, vec![process(10, 5, 100)]);
        let next = sample(start + Duration::from_secs(1), vec![newcomer]);
        assert!(cpu_percent(&prev, &next).is_empty());
    }

    #[test]
    fn is_empty_when_no_wall_time_passed() {
        let start = Instant::now();
        let prev = sample(start, vec![process(10, 5, 100)]);
        let next = sample(start, vec![process(10, 5, 900)]);
        assert!(cpu_percent(&prev, &next).is_empty());
    }

    #[test]
    fn leaves_out_processes_that_ended() {
        let start = Instant::now();
        let prev = sample(start, vec![process(10, 5, 100), process(12, 5, 100)]);
        let next = sample(start + Duration::from_secs(1), vec![process(10, 5, 600)]);
        let percent = cpu_percent(&prev, &next);
        assert_eq!(percent.keys().copied().collect::<Vec<_>>(), vec![10]);
    }
}

mod fake_source {
    use super::*;

    #[test]
    fn hands_out_its_samples_in_order_then_repeats_the_last() {
        let start = Instant::now();
        let first = sample(start, vec![process(1, 1, 0)]);
        let second = sample(start + Duration::from_secs(1), vec![process(2, 1, 0)]);
        let source = FakeSource::new([first, second]);
        let pids = || source.sample().expect("sample").processes.iter().map(|p| p.pid).collect::<Vec<_>>();
        assert_eq!([pids(), pids(), pids()], [vec![1], vec![2], vec![2]]);
    }
}
