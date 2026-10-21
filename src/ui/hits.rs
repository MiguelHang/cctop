use ratatui::layout::{Position, Rect};

use super::dialog::Button;
use super::view::{Sort, ViewTab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    Tab(ViewTab),
    Sort(Sort),
    Row(usize),
    Fold(usize),
    Button(Button),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Hits {
    areas: Vec<(Rect, Hit)>,
}

impl Hits {
    pub(crate) fn at(&self, column: u16, row: u16) -> Option<Hit> {
        let position = Position::new(column, row);
        self.areas.iter().find(|(area, _)| area.contains(position)).map(|(_, hit)| *hit)
    }

    pub(super) fn push(&mut self, area: Rect, hit: Hit) {
        if !area.is_empty() {
            self.areas.push((area, hit));
        }
    }
}
