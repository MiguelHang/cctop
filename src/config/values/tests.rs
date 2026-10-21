use std::time::Duration;

use rstest::rstest;
use toml::Value;

use super::{DURATION, PERCENT, SIZE, SORT};
use crate::config::Sort;

fn value(toml: &str) -> Value {
    toml::from_str::<toml::Table>(&format!("v = {toml}")).expect("valid toml").remove("v").expect("the value")
}

mod durations {
    use super::*;

    #[rstest]
    #[case::seconds(r#""2s""#, Some(Duration::from_secs(2)))]
    #[case::milliseconds(r#""500ms""#, Some(Duration::from_millis(500)))]
    #[case::minutes(r#""1m""#, Some(Duration::from_secs(60)))]
    #[case::minutes_spelled(r#""2min""#, Some(Duration::from_secs(120)))]
    #[case::hours(r#""1h""#, Some(Duration::from_secs(3600)))]
    #[case::fractions(r#""1.5s""#, Some(Duration::from_millis(1500)))]
    #[case::spaces_and_case(r#"" 3 S ""#, Some(Duration::from_secs(3)))]
    #[case::a_bare_integer_is_seconds("4", Some(Duration::from_secs(4)))]
    #[case::a_bare_float_is_seconds("0.25", Some(Duration::from_millis(250)))]
    #[case::not_a_number(r#""x""#, None)]
    #[case::no_unit(r#""2""#, None)]
    #[case::unknown_unit(r#""2d""#, None)]
    #[case::zero(r#""0s""#, None)]
    #[case::the_minimum(r#""100ms""#, Some(Duration::from_millis(100)))]
    #[case::below_the_minimum(r#""99ms""#, None)]
    #[case::a_bare_float_below_the_minimum("0.000001", None)]
    #[case::negative("-1", None)]
    #[case::a_boolean("true", None)]
    fn parse(#[case] toml: &str, #[case] expected: Option<Duration>) {
        assert_eq!((DURATION.parse)(&value(toml)), expected);
    }
}

mod sizes {
    use super::*;

    #[rstest]
    #[case::gibibytes(r#""4GiB""#, Some(4 << 30))]
    #[case::megabytes(r#""512MB""#, Some(512_000_000))]
    #[case::mebibytes(r#""512MiB""#, Some(512 << 20))]
    #[case::kilobytes(r#""2kB""#, Some(2000))]
    #[case::kibibytes(r#""2KiB""#, Some(2048))]
    #[case::gigabytes(r#""5GB""#, Some(5_000_000_000))]
    #[case::terabytes(r#""1TB""#, Some(1_000_000_000_000))]
    #[case::tebibytes(r#""1TiB""#, Some(1 << 40))]
    #[case::bytes(r#""100B""#, Some(100))]
    #[case::fractions(r#""1.5GiB""#, Some(3 << 29))]
    #[case::a_space_and_lower_case(r#""4 gib""#, Some(4 << 30))]
    #[case::a_bare_integer_is_bytes("1024", Some(1024))]
    #[case::not_a_number(r#""lots""#, None)]
    #[case::no_unit(r#""4""#, None)]
    #[case::unknown_unit(r#""4XB""#, None)]
    #[case::zero(r#""0GiB""#, None)]
    #[case::negative("-5", None)]
    #[case::too_big(r#""99999999999TiB""#, None)]
    #[case::a_float("1.5", None)]
    fn parse(#[case] toml: &str, #[case] expected: Option<u64>) {
        assert_eq!((SIZE.parse)(&value(toml)), expected);
    }
}

mod percentages {
    use super::*;

    #[rstest]
    #[case::an_integer("300", Some(300.0))]
    #[case::a_float("150.5", Some(150.5))]
    #[case::a_string_with_the_sign(r#""250%""#, Some(250.0))]
    #[case::a_string_without_it(r#""250""#, Some(250.0))]
    #[case::not_a_number(r#""high""#, None)]
    #[case::zero("0", None)]
    #[case::negative("-10", None)]
    #[case::infinite("inf", None)]
    fn parse(#[case] toml: &str, #[case] expected: Option<f64>) {
        assert_eq!((PERCENT.parse)(&value(toml)), expected);
    }
}

mod sorting {
    use super::*;

    #[rstest]
    #[case::memory(r#""memory""#, Some(Sort::Memory))]
    #[case::cpu(r#""cpu""#, Some(Sort::Cpu))]
    #[case::any_case(r#""CPU""#, Some(Sort::Cpu))]
    #[case::unknown(r#""name""#, None)]
    #[case::not_a_string("1", None)]
    fn parse(#[case] toml: &str, #[case] expected: Option<Sort>) {
        assert_eq!((SORT.parse)(&value(toml)), expected);
    }
}
