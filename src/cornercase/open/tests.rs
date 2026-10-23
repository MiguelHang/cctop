use std::ffi::OsString;
use std::path::Path;

use rstest::rstest;

use super::{clicked, launched, open_cctop, shell_quote};
use crate::cornercase::fake::FakeRunner;
use crate::cornercase::{Client, Target};
use crate::error::Error;

const EXE: &str = "/home/user/.local/share/cornercase/plugins/cctop/target/release/cctop";
const STATUS: [&str; 2] = ["status", "--json"];
const SHELLS: &str = r#"{"id": 10, "program": "zsh"}, {"id": 11, "program": "zsh", "active": true}"#;

type Vars = &'static [(&'static str, &'static str)];

fn env(vars: Vars) -> impl Fn(&str) -> Option<OsString> {
    move |key| vars.iter().find(|(name, _)| *name == key).map(|(_, value)| (*value).into())
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| (*word).to_owned()).collect()
}

fn status(panes: &str) -> String {
    format!(
        r#"{{"projects": [
            {{"id": 1, "workspaces": [{{"id": 1, "tabs": [{{"id": 1, "panes": [{{"id": 1, "program": "zsh"}}]}}]}}]}},
            {{"id": 2, "active": true, "workspaces": [
                {{"id": 3, "tabs": [{{"id": 5, "panes": [{{"id": 6}}, {{"id": 12, "program": "cctop"}}]}}, {{"id": 13}}]}},
                {{"id": 4, "active": true, "tabs": [
                    {{"id": 7, "panes": [{{"id": 8, "program": "zsh"}}, {{"id": 14, "program": "zsh"}}]}},
                    {{"id": 9, "active": true, "panes": [{panes}]}}
                ]}}
            ]}}
        ]}}"#
    )
}

fn split(pane: u64) -> Vec<String> {
    words(&["split", "--pane", &pane.to_string(), "--focus", "--", "exec", EXE])
}

fn run(clicked: Target, panes: &str) -> (crate::error::Result<()>, Vec<Vec<String>>) {
    let runner = (1..=15).fold(FakeRunner::default().answer(&STATUS, &status(panes)), |runner, pane| {
        let id = pane.to_string();
        let split = ["split", "--pane", &id, "--focus", "--", "exec", EXE];
        runner.answer(&split, "20\n").answer(&["focus", "--pane", &id], "")
    });
    let client = Client::new(runner);
    let result = open_cctop(&client, clicked, Path::new(EXE));
    (result, client.runner.calls())
}

mod open_cctop {
    use super::*;

    #[rstest]
    #[case::the_active_pane(11)]
    #[case::a_pane_that_is_not_active(10)]
    fn splits_the_clicked_pane_and_runs_this_binary_in_place_of_its_shell(#[case] pane: u64) {
        let (result, calls) = run(Target::Pane(pane), SHELLS);
        result.expect("open cctop");
        assert_eq!(calls, [words(&STATUS), split(pane)]);
    }

    #[rstest]
    #[case::a_tab(Target::Tab(9), 11)]
    #[case::a_workspace_through_its_active_tab(Target::Workspace(4), 11)]
    #[case::a_project_through_its_active_workspace(Target::Project(2), 11)]
    #[case::a_tab_without_an_active_pane(Target::Tab(7), 8)]
    #[case::a_workspace_without_an_active_tab(Target::Workspace(1), 1)]
    #[case::a_project_without_an_active_workspace(Target::Project(1), 1)]
    fn splits_the_active_or_first_pane_of_what_was_clicked(#[case] clicked: Target, #[case] pane: u64) {
        let (result, calls) = run(clicked, SHELLS);
        result.expect("open cctop");
        assert_eq!(calls, [words(&STATUS), split(pane)]);
    }

    #[rstest]
    #[case::from_another_pane_of_the_tab(Target::Pane(10))]
    #[case::from_the_tab(Target::Tab(9))]
    #[case::from_the_project(Target::Project(2))]
    #[case::from_the_cctop_pane_itself(Target::Pane(15))]
    fn focuses_the_pane_of_that_tab_that_already_runs_cctop(#[case] clicked: Target) {
        let panes = format!(r#"{SHELLS}, {{"id": 15, "program": "/opt/cctop/target/release/cctop"}}"#);
        let (result, calls) = run(clicked, &panes);
        result.expect("open cctop");
        assert_eq!(calls, [words(&STATUS), words(&["focus", "--pane", "15"])]);
    }

    #[rstest]
    #[case::a_cctop_in_another_tab(SHELLS)]
    #[case::a_pane_named_like_it_without_running_it(r#"{"id": 10, "program": "cctop-dev"}, {"id": 11}"#)]
    #[case::panes_that_report_no_program(r#"{"id": 10}, {"id": 11, "active": true}"#)]
    fn splits_when_no_pane_of_the_tab_runs_cctop(#[case] panes: &str) {
        let (result, calls) = run(Target::Pane(11), panes);
        result.expect("open cctop");
        assert_eq!(calls, [words(&STATUS), split(11)]);
    }

    #[rstest]
    #[case::a_pane_status_does_not_list(Target::Pane(99))]
    #[case::a_tab_status_does_not_list(Target::Tab(99))]
    #[case::a_tab_without_panes(Target::Tab(13))]
    #[case::a_workspace_status_does_not_list(Target::Workspace(99))]
    #[case::a_project_status_does_not_list(Target::Project(99))]
    fn fails_without_splitting_when_there_is_no_pane_to_split(#[case] clicked: Target) {
        let (result, calls) = run(clicked, SHELLS);
        let error = result.expect_err("no pane");
        assert!(matches!(error, Error::NoPane(_)), "{error}");
        assert_eq!(calls, [words(&STATUS)]);
    }

    #[test]
    fn stops_when_status_fails() {
        let client = Client::new(FakeRunner::default().fail(&STATUS, "no server"));
        let result = open_cctop(&client, Target::Pane(1), Path::new(EXE));
        assert!(matches!(result, Err(Error::Command { .. })));
        assert_eq!(client.runner.calls(), [words(&STATUS)]);
    }

    #[test]
    fn quotes_a_binary_path_the_shell_would_split() {
        let typed = ["split", "--pane", "11", "--focus", "--", "exec", "'/home/user/My Plugins/cctop'"];
        let client = Client::new(FakeRunner::default().answer(&STATUS, &status(SHELLS)).answer(&typed, "20\n"));
        open_cctop(&client, Target::Pane(11), Path::new("/home/user/My Plugins/cctop")).expect("open cctop");
        assert_eq!(client.runner.calls(), [words(&STATUS), words(&typed)]);
    }
}

mod clicked {
    use super::*;

    const ALL: Vars = &[
        ("CORNERCASE_PROJECT", "2"),
        ("CORNERCASE_WORKSPACE", "4"),
        ("CORNERCASE_TAB", "9"),
        ("CORNERCASE_PANE", "11"),
    ];

    #[rstest]
    #[case::a_pane(ALL, Target::Pane(11))]
    #[case::a_tab(&ALL[..3], Target::Tab(9))]
    #[case::a_workspace(&ALL[..2], Target::Workspace(4))]
    #[case::a_project(&ALL[..1], Target::Project(2))]
    #[case::skipping_ids_that_are_not_numbers(
        &[("CORNERCASE_PANE", ""), ("CORNERCASE_TAB", "nine"), ("CORNERCASE_WORKSPACE", "4")],
        Target::Workspace(4)
    )]
    fn takes_the_narrowest_place_cornercase_names(#[case] vars: Vars, #[case] expected: Target) {
        assert_eq!(clicked(env(vars)).expect("clicked"), expected);
    }

    #[rstest]
    #[case::nothing_set(&[])]
    #[case::only_empty_ids(&[("CORNERCASE_PANE", ""), ("CORNERCASE_PROJECT", "")])]
    fn outside_cornercase_is_a_clear_error(#[case] vars: Vars) {
        let error = clicked(env(vars)).expect_err("nothing clicked");
        assert!(matches!(error, Error::NotInCornercase));
        assert!(error.to_string().contains("CORNERCASE_PANE"), "{error}");
        assert!(error.to_string().contains("cornercase's menu"), "{error}");
    }
}

mod launched {
    use super::*;

    const BUTTON: Vars = &[("CORNERCASE_CONTEXT", "button"), ("CORNERCASE_TAB", "9"), ("CORNERCASE_PANE", "11")];
    const TAB_MENU: Vars = &[("CORNERCASE_CONTEXT", "tab"), ("CORNERCASE_PROJECT", "2"), ("CORNERCASE_TAB", "9")];

    #[rstest]
    #[case::from_the_button(BUTTON, Some(Target::Pane(11)))]
    #[case::from_a_tab_menu(TAB_MENU, Some(Target::Tab(9)))]
    #[case::from_the_button_with_no_project(&[("CORNERCASE_CONTEXT", "button")], None)]
    #[case::in_a_pane_not_run_by_an_action(&[("CORNERCASE_PANE", "11")], None)]
    #[case::with_an_empty_context(&[("CORNERCASE_CONTEXT", ""), ("CORNERCASE_PANE", "11")], None)]
    fn starts_at_the_narrowest_place_of_an_action(#[case] vars: Vars, #[case] expected: Option<Target>) {
        assert_eq!(launched(env(vars)), expected);
    }
}

mod shell_quote {
    use super::*;

    #[rstest]
    #[case::a_plain_path("/opt/cctop/target/release/cctop", "/opt/cctop/target/release/cctop")]
    #[case::a_space("/a b/cctop", "'/a b/cctop'")]
    #[case::a_single_quote("/it's/cctop", r"'/it'\''s/cctop'")]
    #[case::shell_characters("/a$b;c/cctop", "'/a$b;c/cctop'")]
    #[case::empty("", "''")]
    fn keeps_the_path_one_word_for_the_shell(#[case] word: &str, #[case] quoted: &str) {
        assert_eq!(shell_quote(word), quoted);
    }
}
