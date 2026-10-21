use rstest::rstest;

use super::{bytes, estimated, or_missing, percent, thousands, tokens};

mod sizes {
    use super::*;

    #[rstest]
    #[case::zero(0, "0 B")]
    #[case::below_a_kilobyte(1023, "1023 B")]
    #[case::one_kilobyte(1024, "1.0 KB")]
    #[case::megabytes_with_a_decimal(52 * 1024 * 1024 + 300 * 1024, "52.3 MB")]
    #[case::hundreds_without_decimals(512 * 1024 * 1024, "512 MB")]
    #[case::gigabytes(5_583_457_485, "5.2 GB")]
    #[case::terabytes_stay_terabytes(2048 * 1024 * 1024 * 1024 * 1024, "2048 TB")]
    fn are_shown_in_binary_units(#[case] value: u64, #[case] expected: &str) {
        assert_eq!(bytes(value), expected);
    }
}

mod percentages {
    use super::*;

    #[rstest]
    #[case::zero(0.0, "0.0%")]
    #[case::small_with_a_decimal(4.56, "4.6%")]
    #[case::ten_and_above_rounded(42.4, "42%")]
    #[case::several_cores(312.2, "312%")]
    fn are_rounded_by_size(#[case] value: f64, #[case] expected: &str) {
        assert_eq!(percent(value), expected);
    }
}

mod missing_values {
    use super::*;

    #[rstest]
    #[case::size(None, "—")]
    #[case::present(Some(2048), "2.0 KB")]
    fn show_a_dash(#[case] value: Option<u64>, #[case] expected: &str) {
        assert_eq!(or_missing(value, bytes), expected);
    }
}

mod estimates {
    use super::*;

    #[rstest]
    #[case::estimated("5.2 GB", true, "≈5.2 GB")]
    #[case::real("5.2 GB", false, "5.2 GB")]
    #[case::missing_is_never_estimated("—", true, "—")]
    fn are_marked_with_an_approximation_sign(#[case] text: &str, #[case] is_estimate: bool, #[case] expected: &str) {
        assert_eq!(estimated(text.to_owned(), is_estimate), expected);
    }
}

mod token_counts {
    use super::*;

    #[rstest]
    #[case::small(950, "950")]
    #[case::thousands_with_a_decimal(61_535, "61.5k")]
    #[case::hundreds_of_thousands(615_000, "615k")]
    #[case::millions(1_240_000, "1.2M")]
    #[case::whole_millions(1_000_000, "1M")]
    fn are_shortened_by_size(#[case] value: u64, #[case] expected: &str) {
        assert_eq!(tokens(value), expected);
    }

    #[rstest]
    #[case::zero(0, "0")]
    #[case::three_digits(999, "999")]
    #[case::four_digits(1000, "1,000")]
    #[case::millions(1_234_567, "1,234,567")]
    fn are_grouped_by_thousands(#[case] value: u64, #[case] expected: &str) {
        assert_eq!(thousands(value), expected);
    }
}
