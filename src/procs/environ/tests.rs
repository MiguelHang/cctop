use std::path::PathBuf;

use rstest::rstest;

use super::{Env, parse_environ};
use crate::test_util::fixture;

mod environ {
    use super::*;

    #[test]
    fn keeps_only_the_cornercase_and_claude_variables() {
        let expected = Env {
            pane: Some(7),
            server: Some("x-1".to_owned()),
            claude_config_dir: Some(PathBuf::from("/home/user/.claude-work")),
        };
        assert_eq!(parse_environ(fixture!("proc/environ").as_bytes()), expected);
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::cut_in_a_name(b"HOME=/h\0CORNERCASE_PA")]
    #[case::empty_values(b"CORNERCASE_PANE=\0CORNERCASE_SERVER=\0CLAUDE_CONFIG_DIR=\0")]
    #[case::pane_not_a_number(b"CORNERCASE_PANE=seven\0")]
    #[case::no_equals_sign(b"CORNERCASE_SERVER\0")]
    fn leaves_what_is_missing_as_none(#[case] bytes: &[u8]) {
        assert_eq!(parse_environ(bytes), Env::default());
    }
}
