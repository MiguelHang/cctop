use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use rstest::rstest;

use super::{bar, braille, heat, resample, sparkline};

fn symbols(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
}

mod resampling {
    use super::*;

    #[test]
    fn the_newest_point_is_on_the_right_and_missing_slots_are_empty() {
        assert_eq!(resample(&[1.0, 2.0], 4, 4), [None, None, Some(1.0), Some(2.0)]);
    }

    #[test]
    fn several_points_in_a_slot_are_averaged() {
        assert_eq!(resample(&[1.0, 3.0, 5.0, 7.0], 2, 4), [Some(2.0), Some(6.0)]);
    }

    #[test]
    fn fewer_points_than_slots_spread_over_them() {
        assert_eq!(resample(&[1.0, 2.0], 4, 2), [Some(1.0), Some(1.0), Some(2.0), Some(2.0)]);
    }

    #[test]
    fn points_older_than_the_capacity_are_left_out() {
        assert_eq!(resample(&[9.0, 1.0, 2.0], 2, 2), [Some(1.0), Some(2.0)]);
    }

    #[test]
    fn a_point_without_a_reading_is_left_out_of_its_average() {
        assert_eq!(resample(&[None, Some(4.0), None, None], 2, 4), [Some(4.0), None]);
    }

    #[test]
    fn no_slots_give_nothing() {
        assert_eq!(resample(&[1.0], 0, 4), Vec::<Option<f64>>::new());
    }
}

mod drawing {
    use super::*;

    #[test]
    fn braille_fills_each_half_cell_up_to_its_value() {
        let area = Rect::new(0, 0, 2, 1);
        let mut buffer = Buffer::empty(area);
        braille(&mut buffer, area, &[Some(100.0), Some(50.0), Some(0.0), None], 100.0);
        assert_eq!(symbols(&buffer, 0), "⣧ ");
        assert_eq!(buffer[(0, 0)].fg, Color::Red);
    }

    #[test]
    fn braille_spreads_a_value_over_several_rows() {
        let area = Rect::new(0, 0, 1, 2);
        let mut buffer = Buffer::empty(area);
        braille(&mut buffer, area, &[Some(50.0), Some(100.0)], 100.0);
        assert_eq!((symbols(&buffer, 0), symbols(&buffer, 1)), ("⢸".to_owned(), "⣿".to_owned()));
    }

    #[test]
    fn a_tiny_value_still_shows_one_dot() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buffer = Buffer::empty(area);
        braille(&mut buffer, area, &[Some(0.1), None], 100.0);
        assert_eq!(symbols(&buffer, 0), "⡀");
    }

    #[test]
    fn a_sparkline_has_eight_levels_and_blanks_for_nothing() {
        let area = Rect::new(0, 0, 4, 1);
        let mut buffer = Buffer::empty(area);
        sparkline(&mut buffer, area, &[None, Some(0.0), Some(50.0), Some(100.0)], 100.0, 100.0);
        assert_eq!(symbols(&buffer, 0), "  ▄█");
        assert_eq!(buffer[(3, 0)].fg, Color::Red);
    }

    #[rstest]
    #[case::empty(0.0, "░░░░")]
    #[case::half(0.5, "██░░")]
    #[case::an_eighth_of_a_cell(0.53, "██▏░")]
    #[case::full(1.0, "████")]
    #[case::over_full(3.0, "████")]
    fn a_bar_fills_eighths_of_a_cell(#[case] fraction: f64, #[case] expected: &str) {
        let area = Rect::new(0, 0, 4, 1);
        let mut buffer = Buffer::empty(area);
        bar(&mut buffer, area, fraction, Color::Green);
        assert_eq!(symbols(&buffer, 0), expected);
    }

    #[rstest]
    #[case::cool(0.2, Color::Green)]
    #[case::warm(0.6, Color::Yellow)]
    #[case::hot(0.9, Color::Red)]
    fn heat_goes_from_green_to_red(#[case] fraction: f64, #[case] expected: Color) {
        assert_eq!(heat(fraction, 0.5, 0.8), expected);
    }
}
