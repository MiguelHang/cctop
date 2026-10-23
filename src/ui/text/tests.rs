use rstest::rstest;

use super::fit_place;

mod places {
    use super::*;

    #[rstest]
    #[case::whole("shop › main › %3", 20, "shop › main › %3")]
    #[case::without_the_project("shop › main › %3", 12, "main › %3")]
    #[case::the_workspace_cut_at_its_start("shop › a-long-branch › %3", 12, "…branch › %3")]
    #[case::two_parts_without_the_project("a-long-project › main", 12, "main")]
    #[case::two_parts_cut_after_the_project("a-long-project › a-long-branch", 10, "…ng-branch")]
    #[case::the_tab_kept_after_the_workspace("shop › main › review", 15, "main › review")]
    #[case::a_single_part("%3333", 3, "…33")]
    fn drop_the_outer_parts_first(#[case] text: &str, #[case] max: u16, #[case] expected: &str) {
        assert_eq!(fit_place(text, max), expected);
    }
}
