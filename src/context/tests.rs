use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::breakdown::{Accumulator, Breakdown};
use super::record::parse;
use super::{Claude, Sessions};
use crate::test_util::{TempDir, fixture};

const SESSION: &str = fixture!("claude/2.1.306/session.jsonl");
const MINIMAL: &str = fixture!("claude/2.1.306/minimal.jsonl");
const PID: u32 = 4242;

pub(super) fn expected(text: &str) -> Breakdown {
    let mut accumulator = Accumulator::default();
    text.lines().map(parse).for_each(|record| accumulator.push(record));
    accumulator.breakdown()
}

fn transcript_path(config: &Path, id: &str) -> PathBuf {
    config.join("projects").join("-repo").join(format!("{id}.jsonl"))
}

pub(crate) fn start(config: &Path, pid: u32, id: &str, transcript: &str) {
    let sessions = config.join("sessions");
    let project = config.join("projects").join("-repo");
    fs::create_dir_all(&sessions).expect("create sessions folder");
    fs::create_dir_all(&project).expect("create project folder");
    let session = serde_json::json!({ "pid": pid, "sessionId": id, "cwd": "/repo" });
    fs::write(sessions.join(format!("{pid}.json")), session.to_string()).expect("write session file");
    fs::write(project.join(format!("{id}.jsonl")), transcript).expect("write transcript");
}

fn overwrite_first_byte(config: &Path, id: &str) {
    let mut file = OpenOptions::new().write(true).open(transcript_path(config, id)).expect("open transcript");
    file.write_all(b"x").expect("overwrite the first byte");
}

fn claude(config: &Path, pid: u32) -> Claude {
    Claude { pid, config: config.to_path_buf() }
}

fn first_read(config: &Path) -> (Sessions, HashMap<u32, Breakdown>) {
    let mut sessions = Sessions::default();
    start(config, PID, "first", MINIMAL);
    let breakdowns = sessions.read(&[claude(config, PID)]);
    (sessions, breakdowns)
}

mod reading {
    use super::*;

    #[test]
    fn finds_the_transcript_of_each_claude_and_gives_its_breakdown() {
        let config = TempDir::new();
        start(config.path(), PID, "first", SESSION);
        let breakdowns = Sessions::default().read(&[claude(config.path(), PID)]);
        assert_eq!(breakdowns.get(&PID), Some(&expected(SESSION)));
    }

    #[test]
    fn a_claude_without_a_session_or_a_transcript_is_left_out() {
        let config = TempDir::new();
        start(config.path(), PID, "first", SESSION);
        fs::remove_file(transcript_path(config.path(), "first")).expect("delete transcript");
        let breakdowns = Sessions::default().read(&[claude(config.path(), PID), claude(config.path(), 7)]);
        assert!(breakdowns.is_empty());
    }

    #[test]
    fn a_new_session_id_follows_the_new_transcript() {
        let config = TempDir::new();
        let mut sessions = Sessions::default();
        start(config.path(), PID, "first", SESSION);
        sessions.read(&[claude(config.path(), PID)]);
        start(config.path(), PID, "second", MINIMAL);
        let breakdowns = sessions.read(&[claude(config.path(), PID)]);
        assert_eq!(breakdowns.get(&PID), Some(&expected(MINIMAL)));
    }
}

mod caching {
    use super::*;

    #[test]
    fn keeps_one_reader_per_pid_while_it_is_asked_for() {
        let config = TempDir::new();
        let mut sessions = Sessions::default();
        start(config.path(), PID, "first", SESSION);
        start(config.path(), 7, "other", MINIMAL);
        sessions.read(&[claude(config.path(), PID), claude(config.path(), 7)]);
        sessions.read(&[claude(config.path(), PID)]);
        assert_eq!(sessions.readers.keys().copied().collect::<Vec<_>>(), vec![PID]);
    }

    #[test]
    fn a_quiet_tick_reuses_the_reader() {
        let config = TempDir::new();
        let (mut sessions, before) = first_read(config.path());
        overwrite_first_byte(config.path(), "first");
        let breakdowns = sessions.read(&[claude(config.path(), PID)]);
        assert_eq!(breakdowns, before);
    }

    #[test]
    fn a_missing_session_file_keeps_the_reader() {
        let config = TempDir::new();
        let (mut sessions, before) = first_read(config.path());
        let session_file = config.path().join("sessions").join(format!("{PID}.json"));
        let session = fs::read(&session_file).expect("read session file");
        fs::remove_file(&session_file).expect("delete session file");
        assert!(sessions.read(&[claude(config.path(), PID)]).is_empty());
        assert!(sessions.readers.contains_key(&PID));
        fs::write(&session_file, session).expect("restore session file");
        overwrite_first_byte(config.path(), "first");
        assert_eq!(sessions.read(&[claude(config.path(), PID)]), before);
    }
}
