use clap::{CommandFactory, Parser};
use rstest::rstest;

use super::Cli;

mod cli {
    use super::*;

    #[test]
    fn the_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[rstest]
    #[case::no_subcommand_runs_the_tui(&["cctop"], None)]
    #[case::watch(&["cctop", "watch"], Some("Watch"))]
    #[case::open(&["cctop", "open"], Some("Open"))]
    fn parses_the_subcommand(#[case] args: &[&str], #[case] expected: Option<&str>) {
        let cli = Cli::try_parse_from(args).expect("parse arguments");
        assert_eq!(cli.command.map(|command| format!("{command:?}")).as_deref(), expected);
    }

    #[test]
    fn rejects_an_unknown_subcommand() {
        assert!(Cli::try_parse_from(["cctop", "top"]).is_err());
    }

    #[test]
    fn help_lists_watch_and_open() {
        let help = Cli::command().render_long_help().to_string();
        assert!(help.contains("watch") && help.contains("open"), "{help}");
    }
}
