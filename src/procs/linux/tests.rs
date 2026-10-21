use std::fs;
use std::time::Duration;

use rstest::rstest;

use super::{LinuxSource, Stat, current_uid, parse_cmdline, parse_stat, parse_statm, parse_status, swapped};
use crate::procs::ProcessSource;
use crate::procs::probe::own_process;
use crate::test_util::{TempDir, fixture};

const MEMORY_PROBE: &str = "procs::linux::tests::real_proc::memory_probe";

mod status {
    use super::*;

    #[test]
    fn reads_anonymous_memory_plus_swap() {
        assert_eq!(parse_status(fixture!("proc/status")), Some((120_000 + 2048) * 1024));
    }

    #[test]
    fn counts_no_swap_when_the_line_is_missing() {
        assert_eq!(parse_status("RssAnon:\t10 kB\n"), Some(10 * 1024));
    }

    #[rstest]
    #[case::empty("")]
    #[case::cut_in_a_key("Name:\tworker\nPP")]
    #[case::cut_after_the_colon("PPid:\nUid:\nRssAnon:")]
    #[case::not_numbers("PPid:\tx\nUid:\t-\nRssAnon:\tlots kB\n")]
    #[case::only_swap("VmSwap:\t2048 kB\n")]
    fn has_no_memory_when_it_is_missing(#[case] text: &str) {
        assert_eq!(parse_status(text), None);
    }
}

mod statm {
    use super::*;

    #[test]
    fn takes_resident_minus_shared_pages() {
        assert_eq!(parse_statm(b"203080 31000 1000 50 0 40000 0\n", 4096), Some(30_000 * 4096));
    }

    #[rstest]
    #[case::empty(b"")]
    #[case::cut_after_resident(b"203080 31000")]
    #[case::not_numbers(b"a b c d e f g")]
    #[case::no_memory_map(b"0 0 0 0 0 0 0\n")]
    #[case::invalid_utf8(b"\xff 1 1")]
    fn has_no_memory_when_it_is_missing_or_empty(#[case] bytes: &[u8]) {
        assert_eq!(parse_statm(bytes, 4096), None);
    }
}

mod meminfo {
    use super::*;

    #[rstest]
    #[case::no_swap_at_all("SwapTotal:\t0 kB\nSwapFree:\t0 kB\n", false)]
    #[case::swap_unused("MemTotal: 1 kB\nSwapTotal:  2048 kB\nSwapFree:  2048 kB\n", false)]
    #[case::some_swapped_out("SwapTotal:\t2048 kB\nSwapFree:\t1024 kB\n", true)]
    #[case::without_the_swap_lines("MemTotal:\t1024 kB\n", true)]
    #[case::not_numbers("SwapTotal:\tx kB\nSwapFree:\t0 kB\n", true)]
    fn tells_whether_anything_is_swapped_out(#[case] text: &str, #[case] expected: bool) {
        assert_eq!(swapped(text), expected);
    }
}

mod stat {
    use super::*;

    #[test]
    fn takes_the_name_between_the_first_and_last_parenthesis() {
        let expected =
            Stat { name: "tmux: a (b)) c".to_owned(), ppid: 4100, pgid: 4242, cpu_ticks: 375, started: 987_654 };
        assert_eq!(parse_stat(fixture!("proc/stat")), Some(expected));
    }

    #[rstest]
    #[case::empty("")]
    #[case::no_parenthesis("4242 worker S 1 2")]
    #[case::cut_in_the_name("4242 (work")]
    #[case::cut_after_the_name("4242 (worker) S 4100")]
    #[case::cut_before_the_start_time("4242 (worker) S 4100 4242 4100 0 -1 0 0 0 0 0 250 125 0 0 20 0 4 0")]
    #[case::closing_before_opening("1 )x( S")]
    #[case::not_numbers("4242 (worker) S a b c d e f g h i j k l m n o p q r s t")]
    fn is_none_when_truncated(#[case] text: &str) {
        assert_eq!(parse_stat(text), None);
    }
}

mod cmdline {
    use super::*;

    #[rstest]
    #[case::fixture(fixture!("proc/cmdline").as_bytes(), &["/usr/bin/worker", "--flag", "", "last"])]
    #[case::empty(b"", &[])]
    #[case::without_the_last_nul(b"sleep\x0030", &["sleep", "30"])]
    #[case::invalid_utf8(b"a\xff\x00", &["a\u{fffd}"])]
    fn splits_on_nul(#[case] bytes: &[u8], #[case] expected: &[&str]) {
        assert_eq!(parse_cmdline(bytes), expected);
    }
}

mod fake_proc {
    use super::*;

    pub(super) fn write_process(root: &TempDir, pid: &str, files: &[(&str, &str)]) {
        let dir = root.path().join(pid);
        fs::create_dir_all(&dir).expect("create process folder");
        for (name, text) in files {
            fs::write(dir.join(name), text).expect("write process file");
        }
    }

    #[test]
    fn keeps_a_process_whose_name_is_not_utf8() {
        let root = TempDir::new();
        write_process(&root, "4242", &fixture_files()[..1]);
        let stat = fixture!("proc/stat").as_bytes();
        let renamed = [&stat[..6], b"\xff", &stat[10..]].concat();
        fs::write(root.path().join("4242/stat"), renamed).expect("write stat");
        let source = source(&root);
        let process = own_process(&source, 4242).expect("process 4242");
        assert_eq!(process.name, "\u{fffd}: a (b)) c");
    }

    fn source(root: &TempDir) -> LinuxSource {
        LinuxSource::at(root.path().to_owned(), current_uid(), 100)
    }

    pub(super) fn fixture_files() -> [(&'static str, &'static str); 4] {
        [
            ("status", fixture!("proc/status")),
            ("stat", fixture!("proc/stat")),
            ("cmdline", fixture!("proc/cmdline")),
            ("environ", fixture!("proc/environ")),
        ]
    }

    #[test]
    fn reads_a_process_of_the_user() {
        let root = TempDir::new();
        write_process(&root, "4242", &fixture_files());
        let source = source(&root);
        let process = own_process(&source, 4242).expect("process 4242");
        assert_eq!(process.ppid, Some(4100));
        assert_eq!(process.name, "tmux: a (b)) c");
        assert_eq!(process.args[0], "/usr/bin/worker");
        assert_eq!(process.cpu_time, Duration::from_millis(3750));
        assert_eq!((process.pane, process.server.as_deref()), (Some(7), Some("x-1")));
    }

    #[test]
    fn skips_other_users_unreadable_and_non_process_entries() {
        let root = TempDir::new();
        write_process(&root, "4242", &fixture_files());
        write_process(&root, "self", &fixture_files());
        write_process(&root, "5000", &[("status", "Uid:\t1000\n")]);
        write_process(&root, "6000", &[("stat", fixture!("proc/stat"))]);
        let other_user = LinuxSource::at(root.path().to_owned(), current_uid() + 1, 100);
        assert_eq!(other_user.sample().expect("sample").processes, Vec::new());
        let source = source(&root);
        let pids: Vec<_> = source.sample().expect("sample").processes.iter().map(|p| p.pid).collect();
        assert_eq!(pids, vec![4242]);
    }

    #[test]
    fn reads_memory_from_statm_when_nothing_is_swapped_out() {
        let root = TempDir::new();
        write_process(&root, "4242", &fixture_files());
        fs::write(root.path().join("4242/statm"), "203080 31000 1000 50 0 40000 0\n").expect("write statm");
        fs::write(root.path().join("meminfo"), "SwapTotal:\t0 kB\nSwapFree:\t0 kB\n").expect("write meminfo");
        let source = source(&root);
        let memory = own_process(&source, 4242).expect("process 4242").memory;
        assert_eq!(memory, Some(30_000 * super::super::page_size()));
        fs::write(root.path().join("meminfo"), "SwapTotal:\t2048 kB\nSwapFree:\t0 kB\n").expect("write meminfo");
        assert_eq!(own_process(&source, 4242).expect("process 4242").memory, Some((120_000 + 2048) * 1024));
    }

    #[test]
    fn keeps_a_process_without_cmdline_or_environ() {
        let root = TempDir::new();
        write_process(&root, "4242", &fixture_files()[..2]);
        let source = source(&root);
        let process = own_process(&source, 4242).expect("process 4242");
        assert_eq!((process.args.len(), process.pane), (0, None));
    }
}

mod launch_cache {
    use super::*;
    use crate::procs::Process;
    use fake_proc::{fixture_files, write_process};

    fn sampled_twice(root: &TempDir) -> (LinuxSource, Process) {
        write_process(root, "4242", &fixture_files());
        let source = LinuxSource::at(root.path().to_owned(), current_uid(), 100);
        own_process(&source, 4242).expect("first sample");
        let second = own_process(&source, 4242).expect("second sample");
        rewrite_launch(root);
        (source, second)
    }

    fn rewrite_launch(root: &TempDir) {
        fs::write(root.path().join("4242/cmdline"), b"/usr/bin/other\0").expect("write cmdline");
        fs::write(root.path().join("4242/environ"), b"CORNERCASE_PANE=9\0").expect("write environ");
    }

    fn replace_in_stat(root: &TempDir, from: &str, to: &str) {
        let path = root.path().join("4242/stat");
        let stat = fs::read_to_string(&path).expect("read stat").replacen(from, to, 1);
        fs::write(path, stat).expect("write stat");
    }

    #[test]
    fn keeps_the_arguments_and_variables_while_the_process_is_the_same() {
        let root = TempDir::new();
        let (source, first) = sampled_twice(&root);
        fs::write(root.path().join("4242/status"), "PPid:\t4100\nRssAnon:\t10 kB\n").expect("write status");
        let again = own_process(&source, 4242).expect("third sample");
        assert_eq!((&again.args, again.pane), (&first.args, first.pane));
        assert_eq!(again.memory, Some(10 * 1024));
    }

    #[rstest]
    #[case::reused_pid(" 987654 ", " 987999 ")]
    #[case::new_program_in_the_same_process("(tmux: a (b)) c)", "(other)")]
    fn reads_them_again_when_the_start_or_the_name_changes(#[case] from: &str, #[case] to: &str) {
        let root = TempDir::new();
        let (source, _) = sampled_twice(&root);
        replace_in_stat(&root, from, to);
        let again = own_process(&source, 4242).expect("third sample");
        assert_eq!((again.args, again.pane), (vec!["/usr/bin/other".to_owned()], Some(9)));
    }

    #[test]
    fn reads_them_again_on_the_second_sample_of_a_new_process() {
        let root = TempDir::new();
        write_process(&root, "4242", &fixture_files());
        let source = LinuxSource::at(root.path().to_owned(), current_uid(), 100);
        own_process(&source, 4242).expect("first sample");
        rewrite_launch(&root);
        let second = own_process(&source, 4242).expect("second sample");
        assert_eq!((second.args, second.pane), (vec!["/usr/bin/other".to_owned()], Some(9)));
    }

    #[test]
    fn forgets_a_process_that_ended() {
        let root = TempDir::new();
        let (source, _) = sampled_twice(&root);
        fs::rename(root.path().join("4242"), root.path().join("gone")).expect("hide the process");
        assert_eq!(own_process(&source, 4242), None);
        fs::rename(root.path().join("gone"), root.path().join("4242")).expect("bring it back");
        let again = own_process(&source, 4242).expect("third sample");
        assert_eq!(again.pane, Some(9));
    }
}

mod real_proc {
    use super::*;
    use crate::procs::probe;

    #[test]
    fn lists_the_test_process_with_its_parent_and_name() {
        probe::lists_the_test_process(&LinuxSource::new());
    }

    #[test]
    fn reads_the_cornercase_variables_of_a_child() {
        probe::reads_the_variables_of_a_child(&LinuxSource::new());
    }

    #[test]
    fn shows_cpu_after_spinning() {
        probe::shows_cpu_after_spinning(&LinuxSource::new());
    }

    #[test]
    fn counts_anonymous_memory_but_not_a_mapped_file() {
        probe::run_the_memory_probe_alone(MEMORY_PROBE);
    }

    #[test]
    #[ignore = "run alone in its own process by counts_anonymous_memory_but_not_a_mapped_file"]
    fn memory_probe() {
        probe::counts_anonymous_memory_but_not_a_mapped_file(&LinuxSource::new());
    }
}
