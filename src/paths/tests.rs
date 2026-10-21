use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use rstest::rstest;

use super::{config_dir, config_file, state_dir, system_env};

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let vars: HashMap<String, OsString> = vars.iter().map(|(k, v)| ((*k).to_owned(), OsString::from(v))).collect();
    move |key| vars.get(key).cloned()
}

mod config_folder {
    use super::*;

    #[rstest]
    #[case::the_plugin_folder_wins(
        &[("CORNERCASE_PLUGIN_CONFIG", "/plugin/config"), ("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/u")],
        Some("/plugin/config"),
    )]
    #[case::then_xdg_config_home(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/u")], Some("/xdg/cctop"))]
    #[case::then_the_home_config(&[("HOME", "/home/u")], Some("/home/u/.config/cctop"))]
    #[case::empty_values_are_unset(
        &[("CORNERCASE_PLUGIN_CONFIG", ""), ("XDG_CONFIG_HOME", ""), ("HOME", "/home/u")],
        Some("/home/u/.config/cctop"),
    )]
    #[case::relative_values_are_ignored(
        &[("CORNERCASE_PLUGIN_CONFIG", "rel"), ("XDG_CONFIG_HOME", "rel"), ("HOME", "/home/u")],
        Some("/home/u/.config/cctop"),
    )]
    #[case::nothing_without_any_of_them(&[], None)]
    fn follows_the_precedence(#[case] vars: &[(&str, &str)], #[case] expected: Option<&str>) {
        assert_eq!(config_dir(env(vars)), expected.map(PathBuf::from));
    }

    #[test]
    fn the_file_is_config_toml_inside_it() {
        assert_eq!(config_file(env(&[("HOME", "/home/u")])), Some(PathBuf::from("/home/u/.config/cctop/config.toml")));
    }
}

mod state_folder {
    use super::*;

    #[rstest]
    #[case::the_plugin_folder_wins(
        &[("CORNERCASE_PLUGIN_STATE", "/plugin/state"), ("XDG_STATE_HOME", "/xdg"), ("HOME", "/home/u")],
        Some("/plugin/state"),
    )]
    #[case::then_xdg_state_home(&[("XDG_STATE_HOME", "/xdg"), ("HOME", "/home/u")], Some("/xdg/cctop"))]
    #[case::then_the_home_state(&[("HOME", "/home/u")], Some("/home/u/.local/state/cctop"))]
    #[case::empty_values_are_unset(
        &[("CORNERCASE_PLUGIN_STATE", ""), ("XDG_STATE_HOME", ""), ("HOME", "/home/u")],
        Some("/home/u/.local/state/cctop"),
    )]
    #[case::relative_values_are_ignored(
        &[("CORNERCASE_PLUGIN_STATE", "rel"), ("XDG_STATE_HOME", "rel"), ("HOME", "/home/u")],
        Some("/home/u/.local/state/cctop"),
    )]
    #[case::nothing_without_any_of_them(&[], None)]
    fn follows_the_precedence(#[case] vars: &[(&str, &str)], #[case] expected: Option<&str>) {
        assert_eq!(state_dir(env(vars)), expected.map(PathBuf::from));
    }
}

mod system_environment {
    use super::*;

    #[test]
    fn reads_the_process_environment() {
        assert_eq!(system_env("PATH"), std::env::var_os("PATH"));
    }
}
