use rstest::rstest;

use super::Version;

mod parsing {
    use super::*;

    #[rstest]
    #[case::a_tag("v0.2.0", Some("0.2.0"))]
    #[case::a_bare_version("1.10.3", Some("1.10.3"))]
    #[case::with_blanks(" v0.2.0\n", Some("0.2.0"))]
    #[case::a_pre_release("v0.2.0-rc.1", None)]
    #[case::two_parts("v0.2", None)]
    #[case::four_parts("0.2.0.1", None)]
    #[case::a_name("latest", None)]
    #[case::empty("", None)]
    fn reads_only_three_numbers(#[case] text: &str, #[case] expected: Option<&str>) {
        assert_eq!(Version::parse(text).map(|version| version.to_string()).as_deref(), expected);
    }
}

mod ordering {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).expect("a version")
    }

    #[rstest]
    #[case::a_patch("0.1.0", "0.1.1")]
    #[case::a_minor_over_a_larger_patch("0.1.9", "0.2.0")]
    #[case::numbers_not_text("0.9.0", "0.10.0")]
    #[case::a_major("0.99.99", "1.0.0")]
    fn compares_by_number(#[case] older: &str, #[case] newer: &str) {
        assert!(version(older) < version(newer));
    }
}
