const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
const MISSING: &str = "—";
const ESTIMATED: &str = "≈";

#[expect(clippy::cast_precision_loss, reason = "sizes are shown with one decimal at most")]
pub(crate) fn bytes(value: u64) -> String {
    let mut amount = value as f64;
    let mut unit = 0;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else if amount < 100.0 {
        format!("{amount:.1} {}", UNITS[unit])
    } else {
        format!("{amount:.0} {}", UNITS[unit])
    }
}

pub(crate) fn percent(value: f64) -> String {
    if value < 10.0 { format!("{value:.1}%") } else { format!("{value:.0}%") }
}

#[expect(clippy::cast_precision_loss, reason = "token counts are shown with one decimal at most")]
pub(crate) fn tokens(value: u64) -> String {
    let amount = value as f64;
    match value {
        0..1_000 => value.to_string(),
        1_000..100_000 => format!("{:.1}k", amount / 1e3),
        100_000..1_000_000 => format!("{:.0}k", amount / 1e3),
        _ if value.is_multiple_of(1_000_000) => format!("{}M", value / 1_000_000),
        _ => format!("{:.1}M", amount / 1e6),
    }
}

pub(crate) fn or_missing<T>(value: Option<T>, show: impl FnOnce(T) -> String) -> String {
    value.map_or_else(|| MISSING.to_owned(), show)
}

pub(crate) fn estimated(text: String, is_estimate: bool) -> String {
    if is_estimate && text != MISSING { format!("{ESTIMATED}{text}") } else { text }
}

#[cfg(test)]
mod tests;
