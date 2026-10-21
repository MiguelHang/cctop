use std::ops::{Deref, DerefMut};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command};

use rstest::rstest;

use super::{Killer, Signal, Target, own_group};
use crate::error::Error;
use crate::procs::fake::{FakeKiller, Sent};
use crate::procs::identity;
use crate::test_util::wait_until;

struct Reaped(Child);

impl Deref for Reaped {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.0
    }
}

impl DerefMut for Reaped {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}

impl Drop for Reaped {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            self.0.kill().ok();
            self.0.wait().ok();
        }
    }
}

fn sleeper(own_group: bool) -> Reaped {
    let mut command = Command::new("/bin/sleep");
    command.arg("30");
    if own_group {
        command.process_group(0);
    }
    Reaped(command.spawn().expect("spawn /bin/sleep"))
}

fn target_of(child: &Child) -> Target {
    let mut live = None;
    wait_until("the child to show up", || {
        live = identity(child.id());
        live.is_some_and(|live| live.pgid == child.id() || Some(live.pgid) == own_group())
    });
    let live = live.expect("the child is running");
    Target { pid: child.id(), started: live.started, pgid: live.pgid }
}

fn wait_for_sigterm(child: &mut Child) {
    let mut status = None;
    wait_until("the child to end", || {
        status = child.try_wait().expect("poll the child");
        status.is_some()
    });
    assert_eq!(status.and_then(|status| status.signal()), Some(libc::SIGTERM));
}

fn assert_still_running(child: &mut Child) {
    assert_eq!(child.try_wait().expect("poll the child").map(|status| status.signal()), None);
}

mod signal {
    use super::*;

    #[test]
    fn term_ends_a_child_sleep() {
        let mut child = sleeper(false);
        let target = target_of(&child);
        Signal.term(target).expect("send SIGTERM");
        wait_for_sigterm(&mut child);
    }

    #[test]
    fn term_group_ends_a_child_in_its_own_group() {
        let mut child = sleeper(true);
        let target = target_of(&child);
        Signal.term_group(target).expect("send SIGTERM to the group");
        wait_for_sigterm(&mut child);
    }

    #[test]
    fn term_spares_a_process_whose_start_time_differs() {
        let mut child = sleeper(false);
        let target = Target { started: target_of(&child).started + 1, ..target_of(&child) };
        assert!(matches!(Signal.term(target), Err(Error::Ended)));
        assert_still_running(&mut child);
    }

    #[test]
    fn term_spares_a_process_that_changed_group() {
        let mut child = sleeper(false);
        let target = Target { pgid: child.id(), ..target_of(&child) };
        assert!(matches!(Signal.term(target), Err(Error::Ended)));
        assert_still_running(&mut child);
    }

    #[test]
    fn term_group_spares_a_group_whose_leader_started_at_another_time() {
        let mut child = sleeper(true);
        let target = Target { started: target_of(&child).started + 1, ..target_of(&child) };
        assert!(matches!(Signal.term_group(target), Err(Error::Ended)));
        assert_still_running(&mut child);
    }

    #[rstest]
    #[case::pid_zero_would_be_our_own_group(0)]
    #[case::a_pid_beyond_the_kernel_range(u32::MAX)]
    fn term_refuses(#[case] pid: u32) {
        assert!(Signal.term(Target { pid, started: 0, pgid: 1_000 }).is_err());
    }

    #[rstest]
    #[case::group_zero_would_be_our_own(0)]
    #[case::group_one_would_be_every_process(1)]
    fn term_group_refuses(#[case] pgid: u32) {
        assert!(Signal.term_group(Target { pid: 1_000, started: 0, pgid }).is_err());
    }
}

mod fake_killer {
    use super::*;

    #[test]
    fn records_what_it_was_asked_to_send() {
        let killer = FakeKiller::default();
        killer.term(Target { pid: 10, started: 1, pgid: 20 }).expect("term");
        killer.term_group(Target { pid: 10, started: 1, pgid: 20 }).expect("term group");
        assert_eq!(killer.sent(), [Sent::Term(10), Sent::TermGroup(20)]);
    }
}
