use rstest::rstest;

use super::fit_place;

mod places {
    use super::*;

    #[rstest]
    #[case::whole("shop › main › %3", 20, "shop › main › %3")]
    #[case::without_the_project("shop › main › %3", 12, "main › %3")]
    #[case::the_workspace_cut_at_its_start("shop › a-long-branch › %3", 12, "…branch › %3")]
    #[case::a_single_part("%3333", 3, "…33")]
    fn drop_the_outer_parts_first(#[case] text: &str, #[case] max: u16, #[case] expected: &str) {
        assert_eq!(fit_place(text, max), expected);
    }
}
