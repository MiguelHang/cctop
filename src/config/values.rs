use std::time::Duration;

use toml::Value;

use super::Sort;

const MAX_BYTES: f64 = 18_446_744_073_709_549_568.0;
const MIN_DURATION: Duration = Duration::from_millis(100);
const MAX_DURATION: Duration = Duration::from_hours(24);

pub(super) struct Kind<T> {
    pub(super) parse: fn(&Value) -> Option<T>,
    pub(super) expected: &'static str,
}

pub(super) const DURATION: Kind<Duration> =
    Kind { parse: duration, expected: "a duration between 100ms and 24h such as \"2s\"" };
pub(super) const SIZE: Kind<u64> = Kind { parse: size, expected: "a size such as \"4GiB\"" };
pub(super) const PERCENT: Kind<f64> = Kind { parse: percent, expected: "a percentage such as 300" };
pub(super) const SORT: Kind<Sort> = Kind { parse: sort, expected: "\"memory\" or \"cpu\"" };
pub(super) const FLAG: Kind<bool> = Kind { parse: Value::as_bool, expected: "true or false" };

fn duration(value: &Value) -> Option<Duration> {
    let duration = match value {
        Value::Integer(seconds) => Duration::from_secs(u64::try_from(*seconds).ok()?),
        Value::Float(seconds) => Duration::try_from_secs_f64(*seconds).ok()?,
        Value::String(text) => text_duration(text)?,
        _ => return None,
    };
    (MIN_DURATION..=MAX_DURATION).contains(&duration).then_some(duration)
}

fn text_duration(text: &str) -> Option<Duration> {
    let (number, unit) = split(text)?;
    let seconds = match unit.as_str() {
        "ms" => 0.001,
        "s" => 1.0,
        "m" | "min" => 60.0,
        "h" => 3600.0,
        _ => return None,
    };
    Duration::try_from_secs_f64(number * seconds).ok()
}

fn size(value: &Value) -> Option<u64> {
    let bytes = match value {
        Value::Integer(bytes) => u64::try_from(*bytes).ok()?,
        Value::String(text) => text_size(text)?,
        _ => return None,
    };
    (bytes > 0).then_some(bytes)
}

fn text_size(text: &str) -> Option<u64> {
    let (number, unit) = split(text)?;
    let factor = match unit.as_str() {
        "b" => 1.0,
        "kb" => 1e3,
        "mb" => 1e6,
        "gb" => 1e9,
        "tb" => 1e12,
        "kib" => 1024.0,
        "mib" => 1_048_576.0,
        "gib" => 1_073_741_824.0,
        "tib" => 1_099_511_627_776.0,
        _ => return None,
    };
    whole_bytes(number * factor)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is checked to be finite, positive and within u64 first"
)]
fn whole_bytes(bytes: f64) -> Option<u64> {
    let bytes = bytes.round();
    (0.0..=MAX_BYTES).contains(&bytes).then_some(bytes as u64)
}

fn percent(value: &Value) -> Option<f64> {
    let percent = match value {
        Value::Integer(percent) => f64::from(i32::try_from(*percent).ok()?),
        Value::Float(percent) => *percent,
        Value::String(text) => {
            let text = text.trim();
            text.strip_suffix('%').unwrap_or(text).trim_end().parse().ok()?
        }
        _ => return None,
    };
    (percent.is_finite() && percent > 0.0).then_some(percent)
}

fn sort(value: &Value) -> Option<Sort> {
    match value.as_str()?.trim().to_ascii_lowercase().as_str() {
        "memory" => Some(Sort::Memory),
        "cpu" => Some(Sort::Cpu),
        _ => None,
    }
}

fn split(text: &str) -> Option<(f64, String)> {
    let text = text.trim();
    let at = text.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(text.len());
    let (number, unit) = text.split_at(at);
    Some((number.parse().ok()?, unit.trim_start().to_ascii_lowercase()))
}

#[cfg(test)]
mod tests;
