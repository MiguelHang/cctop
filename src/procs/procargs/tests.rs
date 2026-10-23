use std::path::PathBuf;

use rstest::rstest;

use super::{ProcArgs, parse_procargs};
use crate::procs::environ::Env;

fn buffer(argc: i32, rest: &[u8]) -> Vec<u8> {
    [&argc.to_ne_bytes()[..], rest].concat()
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|&arg| arg.to_owned()).collect()
}

mod procargs {
    use super::*;

    #[test]
    fn reads_the_arguments_after_the_padded_exec_path_then_the_environment() {
        let bytes = buffer(
            3,
            b"/bin/sleep\0\0\0\0sleep\0--flag\0\0HOME=/h\0CORNERCASE_PANE=7\0CORNERCASE_SERVER=x-1\0\
              CLAUDE_CONFIG_DIR=/home/user/.claude-work\0executable_path=/bin/sleep\0\0\0",
        );
        let expected = ProcArgs {
            args: args(&["sleep", "--flag", ""]),
            env: Env {
                pane: Some(7),
                server: Some("x-1".to_owned()),
                claude_config_dir: Some(PathBuf::from("/home/user/.claude-work")),
                action_of: None,
            },
        };
        assert_eq!(parse_procargs(&bytes), expected);
    }

    #[test]
    fn decodes_invalid_utf8_lossily() {
        let bytes = buffer(1, b"/a\0a\xff\0CORNERCASE_PANE=3\0");
        assert_eq!(parse_procargs(&bytes).args, args(&["a\u{fffd}"]));
        assert_eq!(parse_procargs(&bytes).env.pane, Some(3));
    }

    #[test]
    fn loses_empty_leading_arguments_as_they_look_like_the_padding() {
        let bytes = buffer(3, b"/a\0\0\0\0x\0CORNERCASE_PANE=7\0");
        let parsed = parse_procargs(&bytes);
        assert_eq!((parsed.args, parsed.env.pane), (args(&["x", "CORNERCASE_PANE=7"]), None));
    }

    #[rstest]
    #[case::argc_larger_than_the_strings(buffer(5, b"/a\0a\0b\0"), &["a", "b"], None)]
    #[case::without_the_final_nul(buffer(2, b"/a\0a\0b"), &["a", "b"], None)]
    #[case::environment_without_the_final_nul(buffer(1, b"/a\0a\0CORNERCASE_PANE=7"), &["a"], Some(7))]
    #[case::no_arguments(buffer(0, b"/a\0\0CORNERCASE_PANE=7\0"), &[], Some(7))]
    #[case::negative_argc(buffer(-1, b"/a\0CORNERCASE_PANE=7\0"), &[], Some(7))]
    #[case::cut_after_the_exec_path(buffer(2, b"/a\0\0\0"), &[], None)]
    #[case::cut_in_the_exec_path(buffer(2, b"/bin/sle"), &[], None)]
    #[case::only_argc(buffer(2, b""), &[], None)]
    #[case::cut_in_argc(vec![2, 0], &[], None)]
    #[case::empty(Vec::new(), &[], None)]
    fn survives_truncated_buffers(#[case] bytes: Vec<u8>, #[case] expected: &[&str], #[case] pane: Option<u64>) {
        let parsed = parse_procargs(&bytes);
        assert_eq!((parsed.args, parsed.env.pane), (args(expected), pane));
    }
}
