use std::collections::HashMap;
use std::time::{Duration, Instant};

use rstest::rstest;

use super::History;
use crate::app::state::RowId;
use crate::app::tests::{STATUS, first_sample, process};
use crate::cornercase::Snapshot;
use crate::procs::{Pid, Sample};
use crate::usage::{self, Tree};

const INTERVAL: Duration = Duration::from_secs(2);

fn tree(sample: &Sample, cpu: &[(Pid, f64)]) -> Tree {
    let snapshot = Snapshot::parse(STATUS).expect("parse the status fixture");
    usage::build(&snapshot, sample, &cpu.iter().copied().collect::<HashMap<_, _>>(), None)
}

fn busy(tick: u32) -> Tree {
    let sample = first_sample(Instant::now());
    tree(&sample, &[(101, f64::from(tick)), (201, 1.0)])
}

mod capacity {
    use super::*;

    #[rstest]
    #[case::every_two_seconds(Duration::from_secs(2), 30, 1800)]
    #[case::every_seven_seconds(Duration::from_secs(7), 9, 515)]
    #[case::slower_than_a_minute(Duration::from_secs(90), 1, 40)]
    #[case::very_fast(Duration::from_millis(100), 600, 3600)]
    #[case::zero(Duration::ZERO, 3600, 3600)]
    fn covers_a_minute_per_row_and_an_hour_in_total(
        #[case] interval: Duration,
        #[case] row: usize,
        #[case] total: usize,
    ) {
        let history = History::new(interval);
        assert_eq!((history.row_points(), history.total_points()), (row, total));
    }

    #[test]
    fn a_row_keeps_only_its_last_minute() {
        let mut history = History::new(INTERVAL);
        (0..100).for_each(|tick| history.record(&busy(tick)));
        let claude = history.row(RowId::Process { pane: 1, pid: 101 });
        assert_eq!(claude.len(), 30);
        assert_eq!(claude.first(), Some(&Some(70.0)));
        assert_eq!(claude.last(), Some(&Some(99.0)));
    }

    #[test]
    fn the_totals_keep_only_their_last_hour() {
        let mut history = History::new(Duration::from_secs(600));
        (0..10).for_each(|tick| history.record(&busy(tick)));
        assert_eq!(history.cpu_share(1.0), [5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        assert_eq!(history.memory().len(), 6);
    }
}

mod rows {
    use super::*;

    #[test]
    fn every_level_has_its_own_line() {
        let mut history = History::new(INTERVAL);
        history.record(&busy(4));
        assert_eq!(history.row(RowId::Process { pane: 1, pid: 101 }), [Some(4.0)]);
        assert_eq!(history.row(RowId::Pane(1)), [Some(4.0)]);
        assert_eq!(history.row(RowId::Pane(2)), [Some(1.0)]);
        assert_eq!(history.row(RowId::Project(1)), [Some(5.0)]);
        assert_eq!(history.row(RowId::Process { pane: 2, pid: 200 }), [Some(1.0)]);
        assert_eq!(history.row(RowId::Process { pane: 7, pid: 300 }), [None]);
    }

    #[test]
    fn rows_that_disappear_are_forgotten() {
        let mut history = History::new(INTERVAL);
        for tick in 0..500 {
            let processes = (0..20).map(|n| process(10_000 + tick * 20 + n, 100, 1, 1, 0)).collect();
            history.record(&tree(&Sample { taken: Instant::now(), processes }, &[]));
        }
        assert_eq!(history.rows(), 13 + 20);
        assert_eq!(history.row(RowId::Process { pane: 1, pid: 10_000 }), Vec::<Option<f64>>::new());
        assert_eq!(history.row(RowId::Pane(1)).len(), 30);
    }

    #[test]
    fn a_row_that_comes_back_starts_again() {
        let mut history = History::new(INTERVAL);
        history.record(&busy(1));
        history.record(&tree(&Sample { taken: Instant::now(), processes: Vec::new() }, &[]));
        history.record(&busy(2));
        assert_eq!(history.row(RowId::Process { pane: 1, pid: 101 }), [Some(2.0)]);
    }

    #[test]
    fn a_tick_without_totals_adds_no_point_to_them() {
        let mut history = History::new(INTERVAL);
        history.record(&tree(&first_sample(Instant::now()), &[]));
        assert_eq!(history.cpu_share(1.0), Vec::<f64>::new());
        assert_eq!(history.memory().len(), 1);
    }
}
