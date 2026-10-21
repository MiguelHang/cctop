use std::path::{Path, PathBuf};

use rstest::rstest;

use super::fake::FakeRunner;
use super::{Client, CommandRunner, Runner, Snapshot, Target};
use crate::error::Error;
use crate::test_util::fixture;

const STATUS: &str = fixture!("cornercase/status-0.12.173.json");
const EXTRA_AND_MISSING: &str = fixture!("cornercase/status-extra-and-missing.json");

fn status() -> Snapshot {
    Snapshot::parse(STATUS).expect("parse the status fixture")
}

fn words(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|word| (*word).to_owned()).collect()
}

mod parsing {
    use super::*;

    #[test]
    fn reads_the_boot_token() {
        assert_eq!(status().boot.as_deref(), Some("4242-1700000000000000000"));
    }

    #[test]
    fn puts_loose_projects_before_grouped_ones_as_the_sidebar_does() {
        let ids: Vec<u64> = status().projects.iter().map(|p| p.id).collect();
        assert_eq!(ids, [1, 2]);
    }

    #[test]
    fn orders_groups_as_listed_and_treats_unlisted_groups_as_loose() {
        let json = r#"{"groups": [{"id": 5}, {"id": 2}], "projects": [
            {"id": 1, "group": 2}, {"id": 2, "group": 5}, {"id": 3}, {"id": 4, "group": 99},
            {"id": 5, "group": 2}, {"id": 6, "group": 5}, {"id": 7}
        ]}"#;
        let ids: Vec<u64> = Snapshot::parse(json).expect("parse").projects.iter().map(|p| p.id).collect();
        assert_eq!(ids, [3, 4, 7, 2, 6, 1, 5]);
    }

    #[test]
    fn keeps_every_workspace_tab_and_pane_in_order() {
        let snapshot = status();
        let tree: Vec<(u64, u64, Vec<u64>)> = snapshot
            .projects
            .iter()
            .flat_map(|p| &p.workspaces)
            .flat_map(|w| w.tabs.iter().map(move |t| (w.id, t.id, t.panes.iter().map(|pane| pane.id).collect())))
            .collect();
        assert_eq!(tree, [(1, 1, vec![1, 2]), (1, 2, vec![]), (2, 3, vec![7]), (3, 4, vec![5])]);
    }

    #[test]
    fn reads_the_project_and_workspace_fields() {
        let snapshot = status();
        let project = &snapshot.projects[0];
        assert_eq!(project.name.as_deref(), Some("notes"));
        assert_eq!(project.path.as_deref(), Some(Path::new("/home/user/src/notes")));
        let worktree = &project.workspaces[1];
        assert_eq!(worktree.name.as_deref(), Some("feature-x"));
        assert_eq!(worktree.branch.as_deref(), Some("feature-x"));
        assert!(worktree.worktree);
        assert!(!project.workspaces[0].worktree);
        assert_eq!(project.workspaces[0].tabs[0].name.as_deref(), Some("claude"));
    }

    #[test]
    fn reads_the_pane_fields() {
        let snapshot = status();
        let pane = &snapshot.projects[0].workspaces[1].tabs[0].panes[0];
        assert_eq!(pane.agent.as_deref(), Some("claude"));
        assert_eq!(pane.status.as_deref(), Some("idle"));
        assert_eq!(pane.model.as_deref(), Some("sonnet"));
        assert_eq!(pane.context_percent, Some(7));
        assert_eq!(pane.subagents, Some(0));
        assert_eq!(pane.parent, Some(1));
        assert_eq!(pane.command.as_deref(), Some("claude"));
        assert_eq!(pane.path, Some(PathBuf::from("/home/user/worktrees/notes/feature-x")));
    }

    #[test]
    fn ignores_unknown_fields_and_leaves_missing_ones_empty() {
        let snapshot = Snapshot::parse(EXTRA_AND_MISSING).expect("parse");
        assert_eq!(snapshot.boot, None);
        let ids: Vec<u64> = snapshot.projects.iter().map(|p| p.id).collect();
        assert_eq!(ids, [9, 13]);
        let project = &snapshot.projects[0];
        assert_eq!((project.name.as_ref(), project.path.as_ref()), (None, None));
        let workspace = &project.workspaces[0];
        assert_eq!((workspace.name.as_ref(), workspace.branch.as_ref(), workspace.worktree), (None, None, false));
        let pane = &workspace.tabs[0].panes[0];
        assert_eq!(pane.id, 12);
        assert_eq!((pane.agent.as_ref(), pane.status.as_ref(), pane.model.as_ref()), (None, None, None));
        assert_eq!((pane.context_percent, pane.subagents, pane.parent), (None, None, None));
        assert_eq!((pane.command.as_ref(), pane.path.as_ref()), (None, None));
        assert_eq!(snapshot.projects[1].workspaces.len(), 0);
    }

    #[rstest]
    #[case::not_json("cornercase is not running")]
    #[case::truncated(r#"{"projects": [{"id": 1"#)]
    #[case::project_without_id(r#"{"projects": [{"name": "x"}]}"#)]
    #[case::pane_id_not_a_number(
        r#"{"projects": [{"id": 1, "workspaces": [{"id": 1, "tabs": [{"id": 1, "panes": [{"id": "one"}]}]}]}]}"#
    )]
    #[case::projects_not_a_list(r#"{"projects": 3}"#)]
    fn rejects_invalid_json(#[case] json: &str) {
        assert!(matches!(Snapshot::parse(json), Err(Error::Json(_))));
    }
}

mod commands {
    use super::*;

    fn client(argv: &[&str], stdout: &str) -> Client<FakeRunner> {
        Client::new(FakeRunner::default().answer(argv, stdout))
    }

    #[test]
    fn status_asks_for_json() {
        let client = client(&["status", "--json"], STATUS);
        assert_eq!(client.status().expect("status"), status());
        assert_eq!(client.runner.calls(), [words(&["status", "--json"])]);
    }

    #[rstest]
    #[case::focused(true, &["open", "/home/user/src/notes", "--focus"])]
    #[case::unfocused(false, &["open", "/home/user/src/notes"])]
    fn open_passes_the_path_and_returns_the_project(#[case] focus: bool, #[case] argv: &[&str]) {
        let client = client(argv, "1\n");
        assert_eq!(client.open(Path::new("/home/user/src/notes"), focus).expect("open"), 1);
        assert_eq!(client.runner.calls(), [words(argv)]);
    }

    #[test]
    fn new_tab_names_the_tab_types_the_command_after_a_separator_and_returns_its_pane() {
        let argv = ["new-tab", "--workspace", "3", "--name", "cctop", "--", "cctop"];
        let client = client(&argv, "8\n");
        assert_eq!(client.new_tab(3, "cctop", &words(&["cctop"])).expect("new-tab"), 8);
        assert_eq!(client.runner.calls(), [words(&argv)]);
    }

    #[rstest]
    #[case::pane(Target::Pane(5), &["focus", "--pane", "5"])]
    #[case::tab(Target::Tab(4), &["focus", "--tab", "4"])]
    #[case::workspace(Target::Workspace(3), &["focus", "--workspace", "3"])]
    #[case::project(Target::Project(2), &["focus", "--project", "2"])]
    fn focus_names_what_it_shows(#[case] target: Target, #[case] argv: &[&str]) {
        let client = client(argv, "");
        client.focus(target).expect("focus");
        assert_eq!(client.runner.calls(), [words(argv)]);
    }

    #[test]
    fn notify_passes_the_text_as_one_argument_after_a_separator() {
        let argv = ["notify", "--", "-5% free, pane 7 uses 2.1 GiB"];
        let client = client(&argv, "");
        client.notify("-5% free, pane 7 uses 2.1 GiB").expect("notify");
        assert_eq!(client.runner.calls(), [words(&argv)]);
    }

    #[test]
    fn a_failing_command_is_an_error() {
        let client = Client::new(FakeRunner::default().fail(&["notify", "--", "hi"], "no server"));
        let Err(Error::Command { argv, stderr }) = client.notify("hi") else { panic!("expected a command error") };
        assert_eq!((argv, stderr.as_str()), (words(&["notify", "--", "hi"]), "no server"));
    }

    #[test]
    fn an_answer_that_is_not_an_id_is_an_error() {
        let client = client(&["open", "/x"], "project opened");
        assert!(matches!(client.open(Path::new("/x"), false), Err(Error::Answer { .. })));
    }

    #[test]
    fn invalid_status_json_is_an_error() {
        assert!(matches!(client(&["status", "--json"], "{").status(), Err(Error::Json(_))));
    }
}

mod command_runner {
    use super::*;

    #[rstest]
    #[case::unset(None)]
    #[case::empty(Some(""))]
    fn defaults_to_cornercase_on_the_path(#[case] bin: Option<&str>) {
        assert_eq!(CommandRunner::new(bin.map(Into::into)).program, "cornercase");
    }

    #[test]
    fn uses_the_binary_cornercase_gives_plugins() {
        assert_eq!(CommandRunner::new(Some("/opt/cc/bin/cornercase".into())).program, "/opt/cc/bin/cornercase");
    }

    #[test]
    fn from_env_always_names_a_program() {
        assert!(!CommandRunner::from_env().program.is_empty());
    }

    #[test]
    fn returns_what_the_program_prints() {
        let runner = CommandRunner::new(Some("sh".into()));
        assert_eq!(runner.run(&words(&["-c", "echo 12"])).expect("run sh"), "12\n");
    }

    #[test]
    fn a_non_zero_exit_is_a_command_error_with_its_stderr() {
        let runner = CommandRunner::new(Some("sh".into()));
        let argv = words(&["-c", "echo no server >&2; exit 3"]);
        let Err(Error::Command { stderr, .. }) = runner.run(&argv) else { panic!("expected a command error") };
        assert_eq!(stderr, "no server");
    }

    #[test]
    fn a_missing_binary_is_an_io_error() {
        let runner = CommandRunner::new(Some("/nonexistent/cornercase".into()));
        assert!(matches!(runner.run(&words(&["status"])), Err(Error::Io(_))));
    }
}
