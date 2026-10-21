use std::time::{Duration, Instant};

use rstest::rstest;

use super::super::fake::{GB, pane, process, tree};
use super::{Kind, Rules, Thresholds};
use crate::usage::Tree;

const THRESHOLDS: Thresholds = Thresholds { memory: 4 * GB, cpu: 300.0, cpu_sustain: Duration::from_secs(60) };

fn with_memory(gb: f64) -> Tree {
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "small test sizes")]
    let bytes = (gb * 1024.0) as u64 * (GB / 1024);
    tree(vec![pane(1, None, vec![process("node", &[], Some(bytes), None)])])
}

fn with_cpu(cpu: Option<f64>) -> Tree {
    tree(vec![pane(1, None, vec![process("node", &[], None, cpu)])])
}

fn alerts_over(trees: &[(u64, Tree)]) -> Vec<(u64, Kind)> {
    let start = Instant::now();
    let mut rules = Rules::new(THRESHOLDS);
    trees
        .iter()
        .flat_map(|(second, tree)| {
            let now = start + Duration::from_secs(*second);
            rules.check(tree, now).into_iter().map(|alert| (*second, alert.kind)).collect::<Vec<_>>()
        })
        .collect()
}

mod memory {
    use super::*;

    #[rstest]
    #[case::stays_over_and_dips_above_the_rearm_line(&[3.0, 5.0, 5.0, 3.5, 5.0], &[1])]
    #[case::drops_below_the_rearm_line_between_crossings(&[3.0, 5.0, 3.0, 5.0], &[1, 3])]
    #[case::starts_over_the_threshold(&[5.0, 5.0], &[0])]
    #[case::never_crosses(&[1.0, 3.9, 4.0], &[])]
    #[case::rearms_just_below_eighty_percent(&[5.0, 3.1, 4.5], &[0, 2])]
    fn alerts_once_per_crossing(#[case] series: &[f64], #[case] expected: &[u64]) {
        let trees: Vec<(u64, Tree)> = (0..).zip(series.iter().map(|gb| with_memory(*gb))).collect();
        let got: Vec<u64> = alerts_over(&trees)
            .into_iter()
            .map(|(at, kind)| {
                assert_eq!(kind, Kind::Memory);
                at
            })
            .collect();
        assert_eq!(got, expected);
    }
}

mod cpu {
    use super::*;

    #[rstest]
    #[case::high_for_59_seconds(&[(0, Some(400.0)), (30, Some(400.0)), (59, Some(400.0))], &[])]
    #[case::high_for_61_seconds(&[(0, Some(400.0)), (30, Some(400.0)), (61, Some(400.0)), (90, Some(400.0))], &[61])]
    #[case::a_dip_under_the_threshold_restarts_the_count(
        &[(0, Some(400.0)), (30, Some(250.0)), (61, Some(400.0)), (100, Some(400.0)), (121, Some(400.0))],
        &[121]
    )]
    #[case::a_dip_above_the_rearm_line_does_not_rearm(
        &[(0, Some(400.0)), (60, Some(400.0)), (70, Some(250.0)), (80, Some(400.0)), (200, Some(400.0))],
        &[60]
    )]
    #[case::a_drop_below_the_rearm_line_rearms(
        &[(0, Some(400.0)), (60, Some(400.0)), (70, Some(100.0)), (80, Some(400.0)), (140, Some(400.0))],
        &[60, 140]
    )]
    #[case::an_unknown_cpu_restarts_the_count(&[(0, Some(400.0)), (30, None), (61, Some(400.0))], &[])]
    fn alerts_once_it_stays_high_for_the_sustain(#[case] series: &[(u64, Option<f64>)], #[case] expected: &[u64]) {
        let trees: Vec<(u64, Tree)> = series.iter().map(|(second, cpu)| (*second, with_cpu(*cpu))).collect();
        let got: Vec<u64> = alerts_over(&trees)
            .into_iter()
            .map(|(at, kind)| {
                assert_eq!(kind, Kind::Cpu);
                at
            })
            .collect();
        assert_eq!(got, expected);
    }
}

mod panes {
    use super::*;

    #[test]
    fn checks_each_pane_on_its_own() {
        let big = || process("node", &[], Some(5 * GB), None);
        let small = || process("zsh", &[], Some(GB), None);
        let first = tree(vec![pane(1, None, vec![big()]), pane(2, None, vec![small()])]);
        let second = tree(vec![pane(1, None, vec![big()]), pane(2, None, vec![big()])]);
        let mut rules = Rules::new(THRESHOLDS);
        let now = Instant::now();
        let mut ids = |tree| rules.check(tree, now).iter().map(|alert| alert.pane.id).collect::<Vec<_>>();
        assert_eq!(ids(&first), [1]);
        assert_eq!(ids(&second), [2]);
    }

    #[test]
    fn forgets_a_pane_that_disappears() {
        let over = tree(vec![pane(1, None, vec![process("node", &[], Some(5 * GB), None)])]);
        let gone = tree(Vec::new());
        let mut rules = Rules::new(THRESHOLDS);
        let now = Instant::now();
        assert_eq!(rules.check(&over, now).len(), 1);
        assert!(rules.check(&gone, now).is_empty());
        assert_eq!(rules.check(&over, now).len(), 1, "a pane that comes back is new");
    }

    #[test]
    fn can_alert_memory_and_cpu_for_the_same_pane() {
        let hot = tree(vec![pane(1, None, vec![process("node", &[], Some(5 * GB), Some(400.0))])]);
        let mut rules = Rules::new(Thresholds { cpu_sustain: Duration::ZERO, ..THRESHOLDS });
        let kinds: Vec<Kind> = rules.check(&hot, Instant::now()).iter().map(|alert| alert.kind).collect();
        assert_eq!(kinds, [Kind::Memory, Kind::Cpu]);
    }
}
