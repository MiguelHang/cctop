use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let mut parts = text.strip_prefix('v').unwrap_or(text).split('.').map(|part| part.parse::<u64>().ok());
        let version = Self { major: parts.next()??, minor: parts.next()??, patch: parts.next()?? };
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests;
