use std::ffi::OsString;
use std::path::Path;

use rstest::rstest;

use super::{open_cctop, state_folder};
use crate::cornercase::Client;
use crate::cornercase::fake::FakeRunner;
use crate::error::Error;
use crate::test_util::TempDir;

const STATE: &str = "/home/user/.local/state/cornercase/plugins/cctop";
const EXE: &str = "/home/user/.local/share/cornercase/plugins/cctop/target/release/cctop";
const OPEN: [&str; 3] = ["open", STATE, "--focus"];
const STATUS: [&str; 2] = ["status", "--json"];
const NEW_TAB: [&str; 7] = ["new-tab", "--workspace", "3", "--name", "cctop", "--", EXE];

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| (*word).to_owned()).collect()
}

fn status(project: u64, path: &str, tabs: &str) -> String {
    format!(
        r#"{{"projects": [
            {{"id": 1, "path": "/home/user/src/notes", "workspaces": [{{"id": 1, "tabs": [
                {{"id": 1, "name": "cctop", "panes": [{{"id": 1, "program": "cctop"}}]}}
            ]}}]}},
            {{"id": {project}, "path": "{path}", "workspaces": [{{"id": 3, "tabs": [{tabs}]}}, {{"id": 4}}]}}
        ]}}"#
    )
}

fn runner(tabs: &str) -> FakeRunner {
    runner_with(&status(2, STATE, tabs))
}

fn runner_with(status: &str) -> FakeRunner {
    runner_at(&OPEN, status)
}

fn runner_at(open: &[&str], status: &str) -> FakeRunner {
    FakeRunner::default()
        .answer(open, "2\n")
        .answer(&STATUS, status)
        .answer(&NEW_TAB, "9\n")
        .answer(&["focus", "--pane", "9"], "")
        .answer(&["focus", "--tab", "5"], "")
}

fn run(runner: FakeRunner) -> (crate::error::Result<()>, Vec<Vec<String>>) {
    let client = Client::new(runner);
    let result = open_cctop(&client, Path::new(STATE), Path::new(EXE));
    (result, client.runner.calls())
}

fn created() -> Vec<Vec<String>> {
    vec![words(&OPEN), words(&STATUS), words(&NEW_TAB), words(&["focus", "--pane", "9"])]
}

fn focused() -> Vec<Vec<String>> {
    vec![words(&OPEN), words(&STATUS), words(&["focus", "--tab", "5"])]
}

mod open_cctop {
    use super::*;

    #[test]
    fn opens_the_state_folder_and_adds_a_tab_running_this_binary_to_a_new_project() {
        let (result, calls) = run(runner(""));
        result.expect("open cctop");
        assert_eq!(calls, created());
    }

    #[rstest]
    #[case::by_its_program(r#"{"id": 5, "name": "top", "panes": [{"id": 6, "program": "cctop"}]}"#)]
    #[case::by_a_program_given_as_a_path(
        r#"{"id": 5, "name": "top", "panes": [{"id": 6, "program": "/opt/bin/cctop"}]}"#
    )]
    #[case::by_its_name_when_no_pane_names_a_program(r#"{"id": 5, "name": "cctop", "panes": [{"id": 6}]}"#)]
    fn focuses_the_tab_that_already_runs_cctop(#[case] tab: &str) {
        let tabs = format!(r#"{{"id": 4, "name": "shell", "panes": [{{"id": 5, "program": "zsh"}}]}}, {tab}"#);
        let (result, calls) = run(runner(&tabs));
        result.expect("open cctop");
        assert_eq!(calls, focused());
    }

    #[rstest]
    #[case::only_a_shell(r#"{"id": 4, "name": "shell", "panes": [{"id": 5, "program": "zsh"}]}"#)]
    #[case::the_cctop_tab_back_at_its_shell(r#"{"id": 4, "name": "cctop", "panes": [{"id": 5, "program": "zsh"}]}"#)]
    #[case::a_tab_without_panes(r#"{"id": 4, "name": "logs", "panes": []}"#)]
    fn adds_the_tab_again_when_none_runs_cctop(#[case] tabs: &str) {
        let (result, calls) = run(runner(tabs));
        result.expect("open cctop");
        assert_eq!(calls, created());
    }

    #[test]
    fn finds_the_project_by_its_path_when_the_ids_differ() {
        let (result, calls) = run(runner_with(&status(7, STATE, "")));
        result.expect("open cctop");
        assert_eq!(calls, created());
    }

    #[test]
    fn finds_the_project_by_its_canonical_path_when_status_spells_it_differently() {
        let temp = TempDir::new();
        std::fs::create_dir_all(temp.path().join("x")).expect("x folder");
        std::fs::create_dir_all(temp.path().join("state")).expect("state folder");
        let state = temp.path().join("state");
        let state_text = state.to_string_lossy().into_owned();
        let spelled = temp.path().join("x/../state").to_string_lossy().into_owned();
        let open = ["open", state_text.as_str(), "--focus"];
        let client = Client::new(runner_at(&open, &status(7, &spelled, "")));
        open_cctop(&client, &state, Path::new(EXE)).expect("open cctop");
        let calls = client.runner.calls();
        assert_eq!(calls, [words(&open), words(&STATUS), words(&NEW_TAB), words(&["focus", "--pane", "9"])]);
    }

    #[test]
    fn fails_without_creating_anything_when_the_project_has_no_workspace() {
        let status = format!(r#"{{"projects": [{{"id": 2, "path": "{STATE}", "workspaces": []}}]}}"#);
        let (result, calls) = run(runner_with(&status));
        assert!(matches!(result, Err(Error::NoWorkspace(path)) if path == Path::new(STATE)));
        assert_eq!(calls, [words(&OPEN), words(&STATUS)]);
    }

    #[test]
    fn fails_without_creating_anything_when_status_lacks_the_project() {
        let (result, calls) = run(runner_with(&status(7, "/somewhere/else", "")));
        let Err(Error::NoWorkspace(path)) = result else { panic!("expected a missing workspace error") };
        assert_eq!(path, Path::new(STATE));
        assert_eq!(calls, [words(&OPEN), words(&STATUS)]);
    }

    #[test]
    fn stops_when_open_fails() {
        let (result, calls) = run(FakeRunner::default().fail(&OPEN, "no server"));
        assert!(matches!(result, Err(Error::Command { .. })));
        assert_eq!(calls, [words(&OPEN)]);
    }
}

mod state_folder {
    use super::*;

    #[rstest]
    #[case::unset(None)]
    #[case::empty(Some(OsString::new()))]
    fn outside_cornercase_is_a_clear_error(#[case] var: Option<OsString>) {
        let error = state_folder(var).expect_err("no state folder");
        assert!(matches!(error, Error::NotInCornercase));
        assert!(error.to_string().contains("CORNERCASE_PLUGIN_STATE"), "{error}");
    }

    #[test]
    fn creates_the_folder_and_returns_its_canonical_path() {
        let temp = TempDir::new();
        let folder = state_folder(Some(temp.path().join("a/../state").into())).expect("state folder");
        assert_eq!(folder, temp.path().join("state"));
        assert!(folder.is_dir());
    }
}
