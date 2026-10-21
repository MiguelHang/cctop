use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::super::tests::expected;
use super::TranscriptReader;
use crate::test_util::{TempDir, fixture};

const SESSION: &str = fixture!("claude/2.1.306/session.jsonl");
const MINIMAL: &str = fixture!("claude/2.1.306/minimal.jsonl");
const PROMPT: &str = "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"one more thing\"}}\n";

fn len(text: &str) -> u64 {
    u64::try_from(text.len()).expect("length fits")
}

fn append(path: &Path, text: &str) {
    let mut file = OpenOptions::new().append(true).open(path).expect("open transcript");
    file.write_all(text.as_bytes()).expect("append to transcript");
}

fn overwrite_first_byte(path: &Path) {
    let mut file = OpenOptions::new().write(true).open(path).expect("open transcript");
    file.write_all(b"x").expect("overwrite the first byte");
}

struct Failing(Option<Vec<u8>>);

impl Read for Failing {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(bytes) = self.0.take() else { return Err(io::Error::other("read failed")) };
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }
}

struct Transcript {
    dir: TempDir,
    path: PathBuf,
}

impl Transcript {
    fn with(text: &str) -> Self {
        let dir = TempDir::new();
        let path = dir.path().join("session.jsonl");
        fs::write(&path, text).expect("write transcript");
        Self { dir, path }
    }

    fn reader(&self) -> TranscriptReader {
        TranscriptReader::new(self.path.clone())
    }
}

mod first_read {
    use super::*;

    #[test]
    fn takes_the_whole_file() {
        let transcript = Transcript::with(SESSION);
        let mut reader = transcript.reader();
        assert_eq!(reader.update(), Some(expected(SESSION)));
        assert_eq!(reader.offset, len(SESSION));
    }

    #[test]
    fn a_missing_file_gives_nothing() {
        let dir = TempDir::new();
        let mut reader = TranscriptReader::new(dir.path().join("missing.jsonl"));
        assert_eq!(reader.update(), None);
    }
}

mod growing_file {
    use super::*;

    #[test]
    fn reads_only_the_new_lines() {
        let transcript = Transcript::with(MINIMAL);
        let mut reader = transcript.reader();
        reader.update();
        overwrite_first_byte(&transcript.path);
        append(&transcript.path, PROMPT);
        let whole = format!("{MINIMAL}{PROMPT}");
        assert_eq!(reader.update(), Some(expected(&whole)));
        assert_eq!(reader.offset, len(&whole));
    }

    #[test]
    fn a_quiet_tick_keeps_the_offset_and_the_breakdown() {
        let transcript = Transcript::with(SESSION);
        let mut reader = transcript.reader();
        reader.update();
        assert_eq!(reader.update(), Some(expected(SESSION)));
        assert_eq!(reader.offset, len(SESSION));
    }
}

mod partial_line {
    use super::*;

    #[test]
    fn is_kept_until_it_is_complete_and_parsed_once() {
        let (head, tail) = MINIMAL.split_at(MINIMAL.len() - 10);
        let complete = &head[..=head.rfind('\n').expect("a complete line")];
        let transcript = Transcript::with(head);
        let mut reader = transcript.reader();
        assert_eq!(reader.update(), Some(expected(complete)));
        assert_eq!(reader.offset, len(head));
        append(&transcript.path, tail);
        assert_eq!(reader.update(), Some(expected(MINIMAL)));
    }

    #[test]
    fn survives_a_failed_read() {
        let (head, tail) = MINIMAL.split_at(MINIMAL.len() - 10);
        let transcript = Transcript::with(head);
        let mut reader = transcript.reader();
        reader.update();
        assert_eq!(reader.consume(Failing(Some(tail.as_bytes().to_vec()))), None);
        assert_eq!(reader.offset, len(head));
        append(&transcript.path, tail);
        assert_eq!(reader.update(), Some(expected(MINIMAL)));
    }
}

mod rereading {
    use super::*;

    #[test]
    fn a_shorter_file_is_read_again_from_the_start() {
        let transcript = Transcript::with(SESSION);
        let mut reader = transcript.reader();
        reader.update();
        fs::write(&transcript.path, MINIMAL).expect("truncate transcript");
        assert_eq!(reader.update(), Some(expected(MINIMAL)));
        assert_eq!(reader.offset, len(MINIMAL));
    }

    #[test]
    fn a_replaced_file_is_read_again_from_the_start() {
        let transcript = Transcript::with(MINIMAL);
        let mut reader = transcript.reader();
        reader.update();
        let whole = format!("{PROMPT}{MINIMAL}");
        let replacement = transcript.dir.path().join("replacement.jsonl");
        fs::write(&replacement, &whole).expect("write replacement");
        fs::rename(&replacement, &transcript.path).expect("replace transcript");
        assert_eq!(reader.update(), Some(expected(&whole)));
        assert_eq!(reader.offset, len(&whole));
    }

    #[test]
    fn a_deleted_file_gives_nothing() {
        let transcript = Transcript::with(SESSION);
        let mut reader = transcript.reader();
        reader.update();
        fs::remove_file(&transcript.path).expect("delete transcript");
        assert_eq!(reader.update(), None);
    }
}
