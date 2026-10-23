use rstest::rstest;

use super::super::record::parse;
use super::{Accumulator, Breakdown, Estimate, Group, Grouped, Part, PartKind, scale};
use crate::test_util::fixture;

const SESSION: &str = fixture!("claude/2.1.306/session.jsonl");
const MINIMAL: &str = fixture!("claude/2.1.306/minimal.jsonl");

fn feed(accumulator: &mut Accumulator, text: &str) {
    text.lines().map(parse).for_each(|record| accumulator.push(record));
}

fn breakdown_of(text: &str) -> Breakdown {
    let mut accumulator = Accumulator::default();
    feed(&mut accumulator, text);
    accumulator.breakdown()
}

fn kinds(parts: &[Part]) -> Vec<PartKind> {
    parts.iter().map(|part| part.kind.clone()).collect()
}

fn sum(parts: &[Part]) -> u64 {
    parts.iter().map(|part| part.tokens.0).sum()
}

fn file(path: &str) -> PartKind {
    PartKind::File(path.to_owned())
}

mod fixed_part {
    use super::*;

    #[test]
    fn lists_the_instruction_files_and_the_rest_of_what_every_turn_pays() {
        let expected = vec![
            PartKind::SystemPrompt,
            PartKind::Instructions("/repo/CLAUDE.md".to_owned()),
            PartKind::Instructions("/repo/AGENTS.md".to_owned()),
            PartKind::Instructions("/repo/DESIGN.md".to_owned()),
            PartKind::Skills,
            PartKind::Tools,
            PartKind::Mcp,
            PartKind::Agents,
        ];
        assert_eq!(kinds(&breakdown_of(SESSION).fixed), expected);
    }

    #[test]
    fn each_instruction_file_weighs_by_its_content() {
        let fixed = breakdown_of(MINIMAL).fixed;
        assert_eq!(
            fixed,
            vec![Part { kind: PartKind::Instructions("/repo/CLAUDE.md".to_owned()), tokens: Estimate(8) }]
        );
    }

    #[test]
    fn the_newest_instructions_replace_the_older_ones() {
        let text = [
            r#"{"type":"attachment","attachment":{"type":"instructions","files":[{"path":"/a.md","content":"aaaa"}]}}"#,
            r#"{"type":"attachment","attachment":{"type":"instructions","files":[{"path":"/b.md","content":"bbbbbbbb"}]}}"#,
        ]
        .join("\n");
        assert_eq!(kinds(&breakdown_of(&text).fixed), vec![PartKind::Instructions("/b.md".to_owned())]);
    }

    #[test]
    fn deltas_add_and_remove_entries_by_name() {
        let text = [
            r#"{"type":"attachment","attachment":{"type":"deferred_tools_delta","addedNames":["aaaa","bbbbbbbb"]}}"#,
            r#"{"type":"attachment","attachment":{"type":"deferred_tools_delta","addedNames":["aaaa"],"removedNames":["bbbbbbbb"]}}"#,
            r#"{"type":"attachment","attachment":{"type":"agent_listing_delta","addedTypes":["x"],"addedLines":["- x"]}}"#,
            r#"{"type":"attachment","attachment":{"type":"agent_listing_delta","removedTypes":["x"]}}"#,
        ]
        .join("\n");
        assert_eq!(breakdown_of(&text).fixed, vec![Part { kind: PartKind::Tools, tokens: Estimate(1) }]);
    }

    #[test]
    fn survives_a_compaction() {
        let text = format!("{MINIMAL}\n{{\"type\":\"system\",\"subtype\":\"compact_boundary\"}}");
        assert_eq!(breakdown_of(&text).fixed, breakdown_of(MINIMAL).fixed);
    }
}

mod dynamic_part {
    use super::*;

    #[test]
    fn only_counts_what_came_after_the_last_compaction() {
        let dynamic = kinds(&breakdown_of(SESSION).dynamic);
        assert!(!dynamic.contains(&file("/repo/src/old.rs")), "{dynamic:?}");
        let mut sorted = dynamic.clone();
        sorted.sort();
        let expected = vec![
            file("/repo/src/main.rs"),
            PartKind::Command("cargo test --locked".to_owned()),
            PartKind::Search("fn main".to_owned()),
            PartKind::Prompts,
            PartKind::Replies,
            PartKind::Subagents,
            PartKind::OtherTools,
        ];
        assert_eq!(sorted, expected);
    }

    #[test]
    fn leaves_sidechain_lines_out() {
        let breakdown = breakdown_of(SESSION);
        assert!(!kinds(&breakdown.dynamic).contains(&file("/repo/src/secret.rs")));
        assert_eq!(breakdown.total, Some(46_203));
    }

    #[test]
    fn is_sorted_from_the_largest_part() {
        let tokens: Vec<u64> = breakdown_of(SESSION).dynamic.iter().map(|part| part.tokens.0).collect();
        assert!(tokens.windows(2).all(|pair| pair[0] >= pair[1]), "{tokens:?}");
    }

    #[rstest]
    #[case::read_joins_use_and_result(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"0123456789012345678901"}]}}"#,
        ],
        &[(file("/a"), 10)],
    )]
    #[case::reads_and_edits_of_one_file_add_up(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a"}},{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"/a"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t2","content":"01234567890123456789"},{"type":"tool_result","tool_use_id":"t1","content":"0123456789"}]}}"#,
        ],
        &[(file("/a"), 17)],
    )]
    #[case::bash_by_command(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"abcdefgh"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"pwd"}}]}}"#,
        ],
        &[(PartKind::Command("ls".to_owned()), 6), (PartKind::Command("pwd".to_owned()), 5)],
    )]
    #[case::grep_and_glob_by_pattern(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Grep","input":{"pattern":"x"}},{"type":"tool_use","id":"t2","name":"Glob","input":{"pattern":"x"}}]}}"#,
        ],
        &[(PartKind::Search("x".to_owned()), 8)],
    )]
    #[case::prompts_and_replies(
        &[
            r#"{"type":"user","message":{"content":"0123456789"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"0123"},{"type":"thinking","thinking":""}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"text","text":"01"}]}}"#,
        ],
        &[(PartKind::Prompts, 3), (PartKind::Replies, 1)],
    )]
    #[case::subagents(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Agent","input":{}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"found it"}]}]}}"#,
        ],
        &[(PartKind::Subagents, 3)],
    )]
    #[case::a_result_without_its_use_is_another_tool(
        &[
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"gone","content":"0123"}]}}"#,
        ],
        &[(PartKind::OtherTools, 1)],
    )]
    #[case::a_result_whose_use_was_compacted_is_another_tool(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a"}}]}}"#,
            r#"{"type":"system","subtype":"compact_boundary"}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"0123"}]}}"#,
        ],
        &[(PartKind::OtherTools, 1)],
    )]
    fn groups_by_file_command_pattern_and_category(#[case] lines: &[&str], #[case] expected: &[(PartKind, u64)]) {
        let parts: Vec<Part> =
            expected.iter().map(|(kind, tokens)| Part { kind: kind.clone(), tokens: Estimate(*tokens) }).collect();
        let breakdown = breakdown_of(&lines.join("\n"));
        assert_eq!((breakdown.total, breakdown.dynamic), (None, parts));
    }
}

mod total {
    use super::*;

    #[test]
    fn the_parts_add_up_to_the_real_total() {
        let breakdown = breakdown_of(SESSION);
        assert_eq!(breakdown.total, Some(46_203));
        assert_eq!(sum(&breakdown.fixed) + sum(&breakdown.dynamic), 46_203);
    }

    #[test]
    fn without_usage_the_parts_are_unscaled_and_the_total_unknown() {
        let breakdown = breakdown_of(MINIMAL);
        assert_eq!(breakdown.total, None);
        assert_eq!(breakdown.dynamic, vec![Part { kind: PartKind::Prompts, tokens: Estimate(3) }]);
    }

    #[test]
    fn a_compaction_forgets_the_total_until_the_next_reply() {
        let text = [
            r#"{"type":"user","message":{"content":"0123"}}"#,
            r#"{"type":"assistant","message":{"content":[],"usage":{"input_tokens":500}}}"#,
            r#"{"type":"system","subtype":"compact_boundary"}"#,
            r#"{"type":"user","message":{"content":"01234567"}}"#,
        ]
        .join("\n");
        let breakdown = breakdown_of(&text);
        assert_eq!(breakdown.total, None);
        assert_eq!(breakdown.dynamic, vec![Part { kind: PartKind::Prompts, tokens: Estimate(2) }]);
    }

    #[test]
    fn a_reply_without_usage_keeps_the_last_total() {
        let text = [
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"0123"}],"usage":{"input_tokens":10}}}"#,
            r#"{"type":"assistant","message":{"model":"<synthetic>","content":[],"usage":{"input_tokens":0}}}"#,
        ]
        .join("\n");
        assert_eq!(breakdown_of(&text).total, Some(10));
    }

    #[rstest]
    #[case::unknown_total_keeps_the_estimates(&[3, 5], None, &[3, 5])]
    #[case::exact_shares(&[1, 3], Some(40), &[10, 30])]
    #[case::largest_remainders_take_the_rounding(&[1, 1, 1], Some(10), &[4, 3, 3])]
    #[case::shrinks_too(&[10, 20, 30], Some(7), &[1, 2, 4])]
    #[case::nothing_estimated_gives_zeros(&[0, 0], Some(100), &[0, 0])]
    #[case::no_parts(&[], Some(100), &[])]
    #[case::huge_numbers(&[u64::MAX / 2, u64::MAX / 2], Some(u64::MAX - 1), &[u64::MAX / 2, u64::MAX / 2])]
    fn scaling_hits_the_total_exactly(#[case] estimates: &[u64], #[case] total: Option<u64>, #[case] expected: &[u64]) {
        assert_eq!(scale(estimates, total), expected);
    }
}

mod tolerance {
    use super::*;

    #[test]
    fn garbage_lines_change_nothing() {
        let garbage =
            "\nnot json\n{\"type\":\n[]\n{\"type\":\"mystery\",\"a\":1}\n{\"type\":\"user\",\"message\":{\"content\":";
        assert_eq!(breakdown_of(&format!("{SESSION}\n{garbage}")), breakdown_of(SESSION));
    }

    #[test]
    fn instructions_without_a_files_array_keep_the_known_files() {
        let broken = r#"{"type":"attachment","attachment":{"type":"instructions","files":"x"}}"#;
        assert_eq!(breakdown_of(&format!("{SESSION}\n{broken}")), breakdown_of(SESSION));
    }

    #[test]
    fn an_empty_transcript_has_nothing() {
        assert_eq!(breakdown_of(""), Breakdown { total: None, fixed: vec![], dynamic: vec![] });
    }
}

mod incremental {
    use super::*;

    #[test]
    fn feeding_in_two_pieces_gives_the_same_breakdown() {
        let whole = breakdown_of(SESSION);
        let lines: Vec<&str> = SESSION.lines().collect();
        for cut in 0..=lines.len() {
            let mut accumulator = Accumulator::default();
            feed(&mut accumulator, &lines[..cut].join("\n"));
            let _ = accumulator.breakdown();
            feed(&mut accumulator, &lines[cut..].join("\n"));
            assert_eq!(accumulator.breakdown(), whole, "cut at line {cut}");
        }
    }
}

mod groups {
    use super::*;

    fn part(kind: PartKind, tokens: u64) -> Part {
        Part { kind, tokens: Estimate(tokens) }
    }

    fn command(text: &str) -> PartKind {
        PartKind::Command(text.to_owned())
    }

    #[test]
    fn join_the_parts_of_a_kind_from_the_largest_group_down() {
        let breakdown = Breakdown {
            total: None,
            fixed: vec![part(PartKind::SystemPrompt, 900)],
            dynamic: vec![
                part(file("src/cart.rs"), 300),
                part(command("cargo test"), 200),
                part(PartKind::Replies, 150),
                part(command("git diff"), 250),
            ],
        };
        let expected = vec![
            Grouped {
                group: Group::Commands,
                tokens: 450,
                parts: vec![part(command("git diff"), 250), part(command("cargo test"), 200)],
            },
            Grouped { group: Group::Files, tokens: 300, parts: vec![part(file("src/cart.rs"), 300)] },
            Grouped { group: Group::Replies, tokens: 150, parts: vec![part(PartKind::Replies, 150)] },
        ];
        assert_eq!(breakdown.groups(), expected);
    }

    #[rstest]
    #[case::file(file("a"), Some(Group::Files))]
    #[case::search(PartKind::Search("fn".to_owned()), Some(Group::Searches))]
    #[case::prompts(PartKind::Prompts, Some(Group::Prompts))]
    #[case::subagents(PartKind::Subagents, Some(Group::Subagents))]
    #[case::other_tools(PartKind::OtherTools, Some(Group::OtherTools))]
    #[case::instructions(PartKind::Instructions("CLAUDE.md".to_owned()), None)]
    #[case::skills(PartKind::Skills, None)]
    fn each_dynamic_kind_has_its_group_and_the_fixed_part_none(
        #[case] kind: PartKind,
        #[case] expected: Option<Group>,
    ) {
        assert_eq!(kind.group(), expected);
    }

    #[test]
    fn the_fixture_s_groups_add_up_to_the_dynamic_part() {
        let breakdown = breakdown_of(SESSION);
        let grouped: u64 = breakdown.groups().iter().map(|grouped| grouped.tokens).sum();
        assert_eq!(grouped, sum(&breakdown.dynamic));
    }
}
