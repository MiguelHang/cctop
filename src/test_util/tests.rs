use std::cell::Cell;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{TempDir, find, wait_until, wait_until_within};

mod temp_dir {
    use super::*;

    #[test]
    fn is_canonical_and_removed_on_drop() {
        let dir = TempDir::new();
        let path = dir.path().to_owned();
        assert_eq!(path.canonicalize().expect("canonicalize"), path);
        assert!(path.is_dir());
        drop(dir);
        assert!(!path.exists());
    }

    #[test]
    fn each_one_is_a_different_folder() {
        assert_ne!(TempDir::new().path(), TempDir::new().path());
    }
}

mod waiting {
    use super::*;

    #[test]
    fn returns_once_the_condition_holds() {
        let calls = Cell::new(0);
        wait_until("three calls", || {
            calls.set(calls.get() + 1);
            calls.get() == 3
        });
        assert_eq!(calls.get(), 3);
    }

    #[test]
    #[should_panic(expected = "timed out waiting for: never")]
    fn panics_after_the_timeout() {
        wait_until_within("never", Duration::from_millis(50), || false);
    }
}

mod fixtures {
    #[test]
    fn reads_from_the_fixtures_folder() {
        assert_eq!(crate::test_util::fixture!(".gitkeep"), "");
    }
}

mod screen {
    use super::*;

    #[test]
    fn finds_text_by_column_and_line() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 2));
        buffer.set_string(0, 1, "→ a b", ratatui::style::Style::new());
        assert_eq!(find(&buffer, "b"), (4, 1));
    }
}
