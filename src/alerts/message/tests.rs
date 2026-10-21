use rstest::rstest;

use super::super::fake::{GB, pane, process, workspace};
use super::super::rules::{Alert, Kind};
use super::message;
use crate::usage::ProcessNode;

fn nested(mut parent: ProcessNode, child: ProcessNode) -> ProcessNode {
    parent.totals.memory = parent.totals.memory.zip(child.totals.memory).map(|(a, b)| a + b);
    parent.totals.cpu = parent.totals.cpu.zip(child.totals.cpu).map(|(a, b)| a + b);
    parent.children.push(child);
    parent
}

fn claude_running_tests() -> Vec<ProcessNode> {
    let test_binary =
        process("shop-3f2a", &["/home/user/src/shop/target/debug/deps/shop-3f2a"], Some(3 * GB), Some(20.0));
    let cargo = nested(process("cargo", &["/usr/bin/cargo", "test"], Some(GB), Some(380.0)), test_binary);
    vec![nested(process("claude", &["claude"], Some(GB), Some(5.0)), cargo)]
}

#[rstest]
#[case::memory(Kind::Memory, "cctop: feat/x › claude 5.0 GB — shop-3f2a 3.0 GB")]
#[case::cpu(Kind::Cpu, "cctop: feat/x › claude 405% — cargo test 380%")]
fn names_the_workspace_the_pane_and_the_heaviest_process(#[case] kind: Kind, #[case] expected: &str) {
    let workspace = workspace(Some("feat/x"), Some("main"), vec![pane(4, Some("claude"), claude_running_tests())]);
    let alert = Alert { kind, workspace: &workspace, pane: &workspace.tabs[0].panes[0] };
    assert_eq!(message(&alert), expected);
}

#[rstest]
#[case::the_branch_without_a_name(None, Some("fix/y"), None, Some("zsh"), "cctop: fix/y › zsh 5.0 GB — node 5.0 GB")]
#[case::ids_without_names(None, None, None, None, "cctop: workspace 7 › pane 4 5.0 GB — node 5.0 GB")]
#[case::the_agent_before_the_program(
    Some("main"),
    None,
    Some("codex"),
    Some("zsh"),
    "cctop: main › codex 5.0 GB — node 5.0 GB"
)]
fn falls_back_when_names_are_missing(
    #[case] name: Option<&str>,
    #[case] branch: Option<&str>,
    #[case] agent: Option<&str>,
    #[case] program: Option<&str>,
    #[case] expected: &str,
) {
    let mut pane = pane(4, agent, vec![process("node", &["node"], Some(5 * GB), None)]);
    pane.command = program.map(str::to_owned);
    let workspace = workspace(name, branch, vec![pane]);
    let alert = Alert { kind: Kind::Memory, workspace: &workspace, pane: &workspace.tabs[0].panes[0] };
    assert_eq!(message(&alert), expected);
}

#[test]
fn shortens_a_long_command_line() {
    let args = ["node", "/home/user/src/shop/node_modules/.bin/vite", "--port", "5173"];
    let workspace =
        workspace(Some("main"), None, vec![pane(4, None, vec![process("node", &args, Some(5 * GB), None)])]);
    let alert = Alert { kind: Kind::Memory, workspace: &workspace, pane: &workspace.tabs[0].panes[0] };
    assert_eq!(message(&alert), "cctop: main › pane 4 5.0 GB — node /home/user/src/shop/node_modules/.… 5.0 GB");
}

#[test]
fn leaves_out_the_process_when_none_has_a_figure() {
    let mut pane = pane(4, None, vec![process("zsh", &["zsh"], None, None)]);
    pane.totals.memory = Some(5 * GB);
    let workspace = workspace(Some("main"), None, vec![pane]);
    let alert = Alert { kind: Kind::Memory, workspace: &workspace, pane: &workspace.tabs[0].panes[0] };
    assert_eq!(message(&alert), "cctop: main › pane 4 5.0 GB");
}

#[rstest]
#[case::memory(Kind::Memory, "cctop: main › pane 4 ≈5.0 GB — node 5.0 GB")]
#[case::cpu(Kind::Cpu, "cctop: main › pane 4 400% — node 400%")]
fn marks_a_partial_memory_total_as_estimated(#[case] kind: Kind, #[case] expected: &str) {
    let mut pane = pane(4, None, vec![process("node", &["node"], Some(5 * GB), Some(400.0))]);
    pane.totals.memory_partial = true;
    let workspace = workspace(Some("main"), None, vec![pane]);
    let alert = Alert { kind, workspace: &workspace, pane: &workspace.tabs[0].panes[0] };
    assert_eq!(message(&alert), expected);
}
