use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{Curl, Fetch, LATEST, Version, cache_file, newer, own};
use crate::error::{Error, Result};
use crate::test_util::TempDir;

const DAY: u64 = 24 * 60 * 60;
const NOW: u64 = 1_800_000_000;

struct FakeFetch {
    answer: std::result::Result<String, String>,
    urls: RefCell<Vec<String>>,
}

impl FakeFetch {
    fn tag(tag: &str) -> Self {
        Self::body(&format!(r#"{{"tag_name": "{tag}", "name": "{tag}", "draft": false}}"#))
    }

    fn body(body: &str) -> Self {
        Self { answer: Ok(body.to_owned()), urls: RefCell::default() }
    }

    fn failing() -> Self {
        Self { answer: Err("Could not resolve host: api.github.com".to_owned()), urls: RefCell::default() }
    }

    fn calls(&self) -> usize {
        self.urls.borrow().len()
    }
}

impl Fetch for FakeFetch {
    fn get(&self, url: &str) -> Result<String> {
        self.urls.borrow_mut().push(url.to_owned());
        self.answer.clone().map_err(|stderr| Error::Command { argv: vec!["curl".to_owned()], stderr })
    }
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn version(text: &str) -> Version {
    Version::parse(text).expect("a version")
}

fn cache(dir: &TempDir) -> PathBuf {
    cache_file(&dir.path().join("state"))
}

fn cached(dir: &TempDir, checked: u64, tag: &str) -> PathBuf {
    let path = cache(dir);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("create the state folder");
    std::fs::write(&path, format!(r#"{{"checked": {checked}, "tag": "{tag}"}}"#)).expect("write the cache");
    path
}

fn check(fetch: &FakeFetch, path: &Path, now: u64) -> Option<String> {
    newer(fetch, path, at(now), version("0.1.0")).map(|newer| newer.to_string())
}

mod asking_github {
    use super::*;

    #[test]
    fn a_newer_release_is_returned_and_kept_for_a_day() {
        let dir = TempDir::new();
        let fetch = FakeFetch::tag("v0.2.0");
        assert_eq!(check(&fetch, &cache(&dir), NOW).as_deref(), Some("0.2.0"));
        assert_eq!(*fetch.urls.borrow(), vec![LATEST.to_owned()]);
        let kept = std::fs::read_to_string(cache(&dir)).expect("the cache is written");
        assert_eq!(kept, format!(r#"{{"checked":{NOW},"tag":"v0.2.0"}}"#));
    }

    #[rstest::rstest]
    #[case::the_same("v0.1.0")]
    #[case::an_older_one("v0.0.9")]
    #[case::a_tag_that_is_not_a_version("nightly")]
    fn nothing_when_the_latest_is_not_newer(#[case] tag: &str) {
        let dir = TempDir::new();
        assert_eq!(check(&FakeFetch::tag(tag), &cache(&dir), NOW), None);
    }

    #[test]
    fn an_answer_that_is_not_a_release_gives_nothing() {
        let dir = TempDir::new();
        assert_eq!(check(&FakeFetch::body(r#"{"message": "API rate limit exceeded"}"#), &cache(&dir), NOW), None);
        assert_eq!(check(&FakeFetch::body("<html>"), &cache(&dir), NOW + 2 * DAY), None);
    }

    #[test]
    fn a_failure_gives_nothing_and_waits_a_day_before_asking_again() {
        let dir = TempDir::new();
        let fetch = FakeFetch::failing();
        assert_eq!(check(&fetch, &cache(&dir), NOW), None);
        assert_eq!(check(&fetch, &cache(&dir), NOW + DAY - 1), None);
        assert_eq!(fetch.calls(), 1);
    }

    #[test]
    fn a_state_folder_that_cannot_be_written_still_answers() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("state"), "a file where the folder goes").expect("block the folder");
        assert_eq!(check(&FakeFetch::tag("v0.2.0"), &cache(&dir), NOW).as_deref(), Some("0.2.0"));
    }
}

mod the_cache {
    use super::*;

    fn asking_for_0_4_0(path: &Path) -> (Option<String>, usize) {
        let fetch = FakeFetch::tag("v0.4.0");
        (check(&fetch, path, NOW), fetch.calls())
    }

    #[rstest::rstest]
    #[case::less_than_a_day_ago(NOW - DAY + 1, (Some("0.3.0"), 0))]
    #[case::a_day_ago(NOW - DAY, (Some("0.4.0"), 1))]
    #[case::in_the_future(NOW + DAY, (Some("0.4.0"), 1))]
    fn a_check_is_repeated_only_after_a_day(#[case] checked: u64, #[case] expected: (Option<&str>, usize)) {
        let dir = TempDir::new();
        let (shown, calls) = asking_for_0_4_0(&cached(&dir, checked, "v0.3.0"));
        assert_eq!((shown.as_deref(), calls), expected);
    }

    #[test]
    fn a_failure_keeps_the_release_known_before() {
        let dir = TempDir::new();
        let path = cached(&dir, NOW - 2 * DAY, "v0.3.0");
        assert_eq!(check(&FakeFetch::failing(), &path, NOW).as_deref(), Some("0.3.0"));
        assert_eq!(check(&FakeFetch::failing(), &path, NOW + DAY - 1).as_deref(), Some("0.3.0"));
    }

    #[test]
    fn a_cache_that_is_not_json_is_asked_again() {
        let dir = TempDir::new();
        let path = cached(&dir, NOW, "v0.3.0");
        std::fs::write(&path, "not json").expect("break the cache");
        assert_eq!(asking_for_0_4_0(&path), (Some("0.4.0".to_owned()), 1));
    }

    #[test]
    fn after_updating_the_cached_release_is_no_longer_newer() {
        let dir = TempDir::new();
        let path = cached(&dir, NOW, "v0.2.0");
        assert_eq!(newer(&FakeFetch::failing(), &path, at(NOW), version("0.2.0")), None);
    }
}

mod curl {
    use super::*;

    #[test]
    fn a_missing_program_is_an_error_not_a_panic() {
        let curl = Curl { program: "cctop-test-no-such-curl" };
        assert!(curl.get(LATEST).is_err());
    }
}

#[test]
fn the_own_version_is_the_crate_s() {
    assert_eq!(own().map(|own| own.to_string()).as_deref(), Some(env!("CARGO_PKG_VERSION")));
}
