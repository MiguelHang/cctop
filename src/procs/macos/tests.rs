use std::time::Duration;

use rstest::rstest;

use super::{MacosSource, Timebase, c_string, machine_memory};
use crate::procs::probe;

const MEMORY_PROBE: &str = "procs::macos::tests::real_processes::memory_probe";

mod timebase {
    use super::*;

    #[rstest]
    #[case::intel(Timebase::new(0, 1, 1), 1_500, Duration::from_nanos(1_500))]
    #[case::apple_silicon(Timebase::new(0, 125, 3), 24_000_000, Duration::from_secs(1))]
    #[case::failed_call(Timebase::new(-1, 125, 3), 7, Duration::from_nanos(7))]
    #[case::zero_denominator(Timebase::new(0, 125, 0), 7, Duration::from_nanos(7))]
    #[case::overflow(Timebase::new(0, u32::MAX, 1), u64::MAX, Duration::from_nanos(u64::MAX))]
    fn converts_ticks_to_time(#[case] timebase: Timebase, #[case] ticks: u64, #[case] expected: Duration) {
        assert_eq!(timebase.to_duration(ticks), expected);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn is_the_apple_silicon_ratio_on_this_machine() {
        assert_eq!(Timebase::current(), Timebase::new(0, 125, 3));
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn is_one_to_one_on_this_intel_machine() {
        assert_eq!(Timebase::current(), Timebase::new(0, 1, 1));
    }
}

mod names {
    use super::*;

    #[rstest]
    #[case::nul_terminated(&[0x73, 0x68, 0, 0x78], "sh")]
    #[case::without_nul(&[0x73, 0x68], "sh")]
    #[case::invalid_utf8(&[0x61, -1, 0], "a\u{fffd}")]
    #[case::empty(&[], "")]
    fn read_up_to_the_first_nul(#[case] chars: &[libc::c_char], #[case] expected: &str) {
        assert_eq!(c_string(chars), expected);
    }
}

mod real_processes {
    use super::*;

    #[test]
    fn lists_the_test_process_with_its_parent_and_name() {
        probe::lists_the_test_process(&MacosSource::new());
    }

    #[test]
    fn reads_the_cornercase_variables_of_a_child() {
        probe::reads_the_variables_of_a_child(&MacosSource::new());
    }

    #[test]
    fn the_machine_holds_more_memory_than_the_test_process() {
        probe::the_machine_holds_the_test_process(&MacosSource::new(), machine_memory());
    }

    #[test]
    fn shows_cpu_after_spinning() {
        probe::shows_cpu_after_spinning(&MacosSource::new());
    }

    #[test]
    fn counts_anonymous_memory_but_not_a_mapped_file() {
        probe::run_the_memory_probe_alone(MEMORY_PROBE);
    }

    #[test]
    #[ignore = "run alone in its own process by counts_anonymous_memory_but_not_a_mapped_file"]
    fn memory_probe() {
        probe::counts_anonymous_memory_but_not_a_mapped_file(&MacosSource::new());
    }
}
