use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use rstest::rstest;

use super::super::sampler::Update;
use super::super::tests::{app_killing_with, click, draw, find, first_sample, index_of, key, mouse, process, reading};
use super::{ENDED, ITSELF, ONLY_PROCESSES};
use crate::app::state::RowId;
use crate::app::{App, Flow};
use crate::error::{Error, Result};
use crate::procs::fake::{FakeKiller, Sent};
use crate::procs::{Killer, Pid, Process, Sample, Target};
use crate::ui::{Button, Dialog};

const CLAUDE: RowId = RowId::Process { pane: 1, pid: 101 };
const OTHER_GROUP: Pid = 999_998;

fn grouped(sample: Sample) -> Sample {
    let group = |pid: Pid| if pid < 200 { 100 } else { pid - pid % 100 };
    let processes =
        sample.processes.into_iter().map(|process| Process { pgid: group(process.pid), ..process }).collect();
    Sample { processes, ..sample }
}

fn sample() -> Sample {
    grouped(first_sample(Instant::now()))
}

fn app(sample: Sample) -> (App, FakeKiller) {
    let killer = FakeKiller::default();
    let mut app = app_killing_with(sample, None, killer.clone());
    app.cctop_group = Some(OTHER_GROUP);
    (app, killer)
}

fn selecting(sample: Sample, id: RowId) -> (App, FakeKiller) {
    let (mut app, killer) = app(sample);
    let index = index_of(&app, id);
    app.state.select(&app.lines, index);
    (app, killer)
}

fn dialog(app: &App) -> Option<Dialog> {
    app.view().dialog
}

fn with_cctop(pgid: Pid) -> Sample {
    let mut sample = sample();
    let cctop = Process { pgid, name: "cctop".to_owned(), ..process(std::process::id(), 100, 1, 10, 0) };
    sample.processes.push(cctop);
    sample
}

mod asking {
    use super::*;

    #[test]
    fn k_opens_a_dialog_naming_the_process_its_pid_and_its_pane() {
        let (mut app, killer) = selecting(sample(), CLAUDE);
        app.handle(&key(KeyCode::Char('k')), Instant::now());
        let expected = Dialog {
            process: "proc101".to_owned(),
            pid: 101,
            pgid: 100,
            pane: "%1 claude".to_owned(),
            focus: Button::Process,
        };
        assert_eq!(dialog(&app), Some(expected));
        assert_eq!(killer.sent(), []);
    }

    #[test]
    fn a_right_click_on_a_process_selects_it_and_asks() {
        let (mut app, _) = app(sample());
        let (_, y) = find(&draw(&mut app, 100, 30), "proc101");
        app.handle(&mouse(MouseEventKind::Down(MouseButton::Right), 40, y), Instant::now());
        assert_eq!(app.state.selected, Some(CLAUDE));
        assert_eq!(dialog(&app).map(|dialog| dialog.pid), Some(101));
    }

    #[test]
    fn a_pane_cannot_be_killed() {
        let (mut app, _) = selecting(sample(), RowId::Pane(1));
        app.handle(&key(KeyCode::Char('k')), Instant::now());
        assert_eq!(dialog(&app), None);
        assert_eq!(app.view().header.error.as_deref(), Some(ONLY_PROCESSES));
    }

    #[test]
    fn cctop_itself_is_refused_with_a_warning() {
        let (mut app, killer) = selecting(with_cctop(999_999), RowId::Process { pane: 1, pid: std::process::id() });
        app.handle(&key(KeyCode::Char('k')), Instant::now());
        assert_eq!(dialog(&app), None);
        assert_eq!(app.view().header.error.as_deref(), Some(ITSELF));
        assert_eq!(killer.sent(), []);
    }

    #[rstest]
    #[case::a_process_in_the_group_of_cctop(Some(100))]
    #[case::any_process_when_the_group_of_cctop_is_unknown(None)]
    fn is_refused_too(#[case] cctop_group: Option<Pid>) {
        let (mut app, _) = selecting(sample(), CLAUDE);
        app.cctop_group = cctop_group;
        app.handle(&key(KeyCode::Char('k')), Instant::now());
        assert_eq!(dialog(&app), None);
        assert_eq!(app.view().header.error.as_deref(), Some(ITSELF));
    }
}

mod answering {
    use super::*;

    fn asked() -> (App, FakeKiller) {
        let (mut app, killer) = selecting(sample(), CLAUDE);
        app.handle(&key(KeyCode::Char('k')), Instant::now());
        (app, killer)
    }

    fn click_on(app: &mut App, text: &str) {
        let (x, y) = find(&draw(app, 100, 30), text);
        app.handle(&click(x, y), Instant::now());
    }

    #[test]
    fn nothing_is_killed_without_confirming() {
        let (mut app, killer) = asked();
        assert_eq!(app.handle(&key(KeyCode::Esc), Instant::now()), Flow::Continue);
        assert_eq!(dialog(&app), None);
        app.handle(&key(KeyCode::Char('k')), Instant::now());
        click_on(&mut app, "[Cancel]");
        assert_eq!(dialog(&app), None);
        assert_eq!(killer.sent(), []);
    }

    #[test]
    fn enter_kills_the_process() {
        let (mut app, killer) = asked();
        app.handle(&key(KeyCode::Enter), Instant::now());
        assert_eq!(killer.sent(), [Sent::Term(101)]);
        assert_eq!(dialog(&app), None);
    }

    #[test]
    fn clicking_group_kills_the_group() {
        let (mut app, killer) = asked();
        click_on(&mut app, "[Group]");
        assert_eq!(killer.sent(), [Sent::TermGroup(100)]);
    }

    #[test]
    fn arrows_move_the_focus_that_enter_uses() {
        let (mut app, killer) = asked();
        app.handle(&key(KeyCode::Right), Instant::now());
        assert_eq!(dialog(&app).map(|dialog| dialog.focus), Some(Button::Group));
        app.handle(&key(KeyCode::Right), Instant::now());
        app.handle(&key(KeyCode::Enter), Instant::now());
        assert_eq!(killer.sent(), []);
    }

    #[test]
    fn while_open_no_click_or_key_reaches_the_tree() {
        let (mut app, _) = asked();
        let (_, y) = find(&draw(&mut app, 100, 30), "%2 cargo");
        app.handle(&click(5, y), Instant::now());
        assert_eq!(app.handle(&key(KeyCode::Char('q')), Instant::now()), Flow::Continue);
        app.handle(&key(KeyCode::Down), Instant::now());
        assert_eq!(app.state.selected, Some(CLAUDE));
        assert!(dialog(&app).is_some());
        assert_eq!(
            app.handle(&Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), Instant::now()),
            Flow::Quit
        );
    }

    #[test]
    fn a_process_that_ended_meanwhile_is_not_killed() {
        let (mut app, killer) = asked();
        let mut next = sample();
        next.processes.retain(|process| process.pid != 101);
        app.update(Update::Reading(Ok(reading(next))), Instant::now());
        app.handle(&key(KeyCode::Enter), Instant::now());
        assert_eq!(killer.sent(), []);
        assert_eq!(app.view().header.error.as_deref(), Some(ENDED));
    }

    #[test]
    fn a_reused_pid_is_not_killed() {
        let (mut app, killer) = asked();
        let mut next = sample();
        next.processes.iter_mut().filter(|process| process.pid == 101).for_each(|process| process.started = 7);
        app.update(Update::Reading(Ok(reading(next))), Instant::now());
        app.handle(&key(KeyCode::Enter), Instant::now());
        assert_eq!(killer.sent(), []);
    }

    #[test]
    fn a_process_that_changed_group_is_not_killed() {
        let (mut app, killer) = asked();
        let mut next = sample();
        next.processes.iter_mut().filter(|process| process.pid == 101).for_each(|process| process.pgid = 101);
        app.update(Update::Reading(Ok(reading(next))), Instant::now());
        click_on(&mut app, "[Group]");
        assert_eq!(killer.sent(), []);
        assert_eq!(app.view().header.error.as_deref(), Some(ENDED));
    }

    #[test]
    fn the_group_of_cctop_is_checked_again_on_confirming() {
        let (mut app, killer) = asked();
        app.cctop_group = Some(100);
        click_on(&mut app, "[Group]");
        assert_eq!(killer.sent(), []);
        assert_eq!(app.view().header.error.as_deref(), Some(ITSELF));
    }

    #[test]
    fn the_killer_gets_the_identity_seen_in_the_sample() {
        let (mut app, _) = asked();
        let seen = Rc::new(RefCell::new(Vec::new()));
        app.killer = Box::new(Recording(Rc::clone(&seen)));
        app.handle(&key(KeyCode::Enter), Instant::now());
        let claude = sample().processes.into_iter().find(|process| process.pid == 101).expect("claude");
        assert_eq!(*seen.borrow(), [Target { pid: 101, started: claude.started, pgid: 100 }]);
    }

    #[test]
    fn a_process_the_killer_finds_ended_shows_a_warning() {
        let (mut app, _) = asked();
        app.killer = Box::new(Ended);
        app.handle(&key(KeyCode::Enter), Instant::now());
        assert_eq!(app.view().header.error.as_deref(), Some(ENDED));
    }

    struct Recording(Rc<RefCell<Vec<Target>>>);

    impl Killer for Recording {
        fn term(&self, target: Target) -> Result<()> {
            self.0.borrow_mut().push(target);
            Ok(())
        }

        fn term_group(&self, target: Target) -> Result<()> {
            self.term(target)
        }
    }

    struct Ended;

    impl Killer for Ended {
        fn term(&self, _: Target) -> Result<()> {
            Err(Error::Ended)
        }

        fn term_group(&self, target: Target) -> Result<()> {
            self.term(target)
        }
    }

    struct Refusing;

    impl Killer for Refusing {
        fn term(&self, _: Target) -> Result<()> {
            Err(std::io::Error::from_raw_os_error(libc::EPERM).into())
        }

        fn term_group(&self, target: Target) -> Result<()> {
            self.term(target)
        }
    }

    #[test]
    fn a_failed_kill_is_reported() {
        let (mut app, _) = asked();
        app.killer = Box::new(Refusing);
        app.handle(&key(KeyCode::Enter), Instant::now());
        let error = app.view().header.error.expect("an error is shown");
        assert!(error.starts_with("cannot kill proc101: "), "{error}");
    }
}
