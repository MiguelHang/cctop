use std::collections::HashSet;

use ratatui::text::Span;
use rstest::rstest;
use toml::{Table, Value};

const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/cornercase-plugin.toml"));
const FIRST_WITH_BUTTONS: [u64; 3] = [0, 12, 187];
const CONTEXTS: &[&str] = &["project", "workspace", "tab", "pane", "button"];
const WIDTH: std::ops::RangeInclusive<i64> = 20..=500;
const HEIGHT: std::ops::RangeInclusive<i64> = 5..=200;

fn manifest() -> Table {
    MANIFEST.parse().expect("the manifest is valid TOML")
}

fn actions(manifest: &Table) -> Vec<&Table> {
    let actions = manifest.get("actions").and_then(Value::as_array).expect("the manifest has actions");
    actions.iter().map(|action| action.as_table().expect("an action is a table")).collect()
}

fn text<'a>(table: &'a Table, key: &str) -> Option<&'a str> {
    table.get(key).and_then(Value::as_str)
}

fn strings<'a>(table: &'a Table, key: &str) -> Vec<&'a str> {
    let values = table.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    values.iter().map(|value| value.as_str().expect("a list of strings")).collect()
}

fn action<'a>(manifest: &'a Table, id: &str) -> &'a Table {
    actions(manifest).into_iter().find(|action| text(action, "id") == Some(id)).expect("the action exists")
}

fn version(text: &str) -> Vec<u64> {
    text.split('.').map(|part| part.parse().expect("a numeric version")).collect()
}

fn hidden(text: &str) -> bool {
    text.chars().any(|c| {
        c.is_control()
            || matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
    })
}

mod for_cornercase_0_12_187 {
    use super::*;

    #[test]
    fn asks_for_the_first_cornercase_with_buttons_and_popups() {
        let manifest = manifest();
        let min = text(&manifest, "min_cornercase").expect("min_cornercase is set");
        assert!(version(min) >= FIRST_WITH_BUTTONS.to_vec(), "{min}");
    }

    #[test]
    fn the_button_has_a_title_and_an_icon_of_one_cell() {
        let manifest = manifest();
        let button = manifest.get("button").and_then(Value::as_table).expect("a [button] table");
        assert!(text(button, "title").is_some_and(|title| !title.trim().is_empty()));
        let icon = text(button, "icon").expect("an icon");
        assert_eq!(icon.chars().count(), 1, "{icon}");
        assert_eq!(Span::raw(icon).width(), 1, "{icon}");
        assert!(!hidden(icon));
    }

    #[test]
    fn an_action_lands_on_the_button() {
        let manifest = manifest();
        assert!(actions(&manifest).iter().any(|action| strings(action, "contexts").contains(&"button")));
    }

    #[test]
    fn every_action_has_a_unique_id_a_title_known_contexts_and_a_command() {
        let manifest = manifest();
        let mut ids = HashSet::new();
        for action in actions(&manifest) {
            let id = text(action, "id").expect("an id");
            assert!(ids.insert(id), "two actions are called {id}");
            assert!(text(action, "title").is_some_and(|title| !title.trim().is_empty()), "{id}");
            let contexts = strings(action, "contexts");
            assert!(!contexts.is_empty() && contexts.iter().all(|context| CONTEXTS.contains(context)), "{id}");
            let command = strings(action, "command");
            assert!(!command.is_empty() && !command.iter().any(|word| hidden(word)), "{id}");
        }
    }

    #[test]
    fn only_a_popup_has_a_size_and_it_is_in_range() {
        let manifest = manifest();
        for action in actions(&manifest) {
            let placement = text(action, "placement").unwrap_or("job");
            assert!(["job", "popup"].contains(&placement), "{placement}");
            let sizes = [("width", WIDTH), ("height", HEIGHT)].into_iter();
            for (key, range, size) in sizes.filter_map(|(key, range)| Some((key, range, action.get(key)?))) {
                assert_eq!(placement, "popup", "{key} without a popup");
                assert!(size.as_integer().is_some_and(|size| range.contains(&size)), "{key} = {size}");
            }
        }
    }
}

mod actions {
    use super::*;

    #[test]
    fn the_button_opens_the_tui_in_a_popup_of_the_default_size() {
        let manifest = manifest();
        let popup = action(&manifest, "cctop");
        assert_eq!(text(popup, "placement"), Some("popup"));
        assert_eq!(strings(popup, "command"), ["target/release/cctop"]);
        assert!(popup.get("width").is_none() && popup.get("height").is_none());
    }

    #[rstest]
    #[case::button("button")]
    #[case::project("project")]
    #[case::workspace("workspace")]
    #[case::tab("tab")]
    #[case::pane("pane")]
    fn the_popup_is_offered(#[case] context: &str) {
        assert!(strings(action(&manifest(), "cctop"), "contexts").contains(&context));
    }

    #[test]
    fn the_pane_menu_keeps_cctop_in_a_split() {
        let manifest = manifest();
        let split = action(&manifest, "split");
        assert_eq!(strings(split, "contexts"), ["pane"]);
        assert_eq!(strings(split, "command"), ["target/release/cctop", "open"]);
        assert!(split.get("placement").is_none());
    }
}
