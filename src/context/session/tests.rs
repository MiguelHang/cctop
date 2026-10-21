use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use rstest::rstest;

use super::{Session, config_dir, project_dir, session, transcript};
use crate::test_util::TempDir;

const ID: &str = "00000000-0000-4000-8000-000000000001";

fn write_session(config: &Path, pid: u32, text: &str) {
    let sessions = config.join("sessions");
    fs::create_dir_all(&sessions).expect("create sessions folder");
    fs::write(sessions.join(format!("{pid}.json")), text).expect("write session file");
}

fn write_transcript(config: &Path, folder: &str) -> PathBuf {
    let folder = config.join("projects").join(folder);
    fs::create_dir_all(&folder).expect("create project folder");
    let path = folder.join(format!("{ID}.jsonl"));
    fs::write(&path, "").expect("write transcript");
    path
}

fn session_in(cwd: &str) -> Session {
    Session { id: ID.to_owned(), cwd: PathBuf::from(cwd) }
}

mod config_folder {
    use super::*;

    #[rstest]
    #[case::the_variable_wins(Some("/claude"), Some("/home/u"), Some("/claude"))]
    #[case::home_by_default(None, Some("/home/u"), Some("/home/u/.claude"))]
    #[case::a_relative_variable_is_ignored(Some("claude"), Some("/home/u"), Some("/home/u/.claude"))]
    #[case::nothing_without_home(None, None, None)]
    fn comes_from_the_environment(
        #[case] variable: Option<&str>,
        #[case] home: Option<&str>,
        #[case] expected: Option<&str>,
    ) {
        let env = |key: &str| match key {
            "CLAUDE_CONFIG_DIR" => variable.map(OsString::from),
            "HOME" => home.map(OsString::from),
            _ => None,
        };
        assert_eq!(config_dir(env), expected.map(PathBuf::from));
    }
}

mod session_file {
    use super::*;

    #[test]
    fn gives_the_session_id_and_the_folder() {
        let config = TempDir::new();
        write_session(config.path(), 42, &format!(r#"{{"pid":42,"sessionId":"{ID}","cwd":"/repo","status":"idle"}}"#));
        assert_eq!(session(config.path(), 42), Some(session_in("/repo")));
    }

    #[rstest]
    #[case::not_json("{")]
    #[case::without_an_id(r#"{"pid":42,"cwd":"/repo"}"#)]
    #[case::without_a_folder(r#"{"pid":42,"sessionId":"abc"}"#)]
    #[case::with_an_id_that_leaves_the_folder(r#"{"pid":42,"sessionId":"../abc","cwd":"/repo"}"#)]
    fn gives_nothing_when_it_is_broken(#[case] text: &str) {
        let config = TempDir::new();
        write_session(config.path(), 42, text);
        assert_eq!(session(config.path(), 42), None);
    }

    #[test]
    fn gives_nothing_when_it_is_missing() {
        let config = TempDir::new();
        assert_eq!(session(config.path(), 42), None);
    }
}

mod project_folder {
    use super::*;

    #[rstest]
    #[case::every_non_alphanumeric_becomes_a_dash("/home/u/my app.v2", "-home-u-my-app-v2")]
    #[case::one_dash_per_utf16_unit("/ñ/😀", "-----")]
    fn follows_claude_s_rule(#[case] cwd: &str, #[case] expected: &str) {
        assert_eq!(project_dir(Path::new(cwd)), expected);
    }

    #[test]
    fn a_long_path_is_cut_and_hashed() {
        let cwd = format!("/{}", "a".repeat(250));
        assert_eq!(project_dir(Path::new(&cwd)), format!("-{}-feo44x", "a".repeat(199)));
    }
}

mod transcript_file {
    use super::*;

    #[test]
    fn sits_in_the_project_folder() {
        let config = TempDir::new();
        let path = write_transcript(config.path(), "-repo");
        assert_eq!(transcript(config.path(), &session_in("/repo")), Some(path));
    }

    #[test]
    fn a_long_path_is_found_by_listing_the_projects() {
        let config = TempDir::new();
        fs::create_dir_all(config.path().join("projects").join("-elsewhere")).expect("create project folder");
        let path = write_transcript(config.path(), &format!("-{}-otherhash", "a".repeat(199)));
        let cwd = format!("/{}", "a".repeat(250));
        assert_eq!(transcript(config.path(), &session_in(&cwd)).as_deref(), Some(path.as_path()));
    }

    #[test]
    fn gives_nothing_when_no_project_has_it() {
        let config = TempDir::new();
        fs::create_dir_all(config.path().join("projects").join("-repo")).expect("create project folder");
        assert_eq!(transcript(config.path(), &session_in("/repo")), None);
    }

    #[test]
    fn gives_nothing_without_projects() {
        let config = TempDir::new();
        assert_eq!(transcript(config.path(), &session_in("/repo")), None);
    }
}
