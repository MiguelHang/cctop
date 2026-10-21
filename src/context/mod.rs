#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by the transcript reader and the context view, tasks 007 and 012")
)]
pub(crate) mod breakdown;
mod reader;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by the transcript reader and the context view, tasks 007 and 012")
)]
pub(crate) mod record;
pub(crate) mod session;

use std::collections::HashMap;
use std::path::PathBuf;

use self::breakdown::Breakdown;
use self::reader::TranscriptReader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Claude {
    pub(crate) pid: u32,
    pub(crate) config: PathBuf,
}

#[derive(Debug, Default)]
pub(crate) struct Sessions {
    readers: HashMap<u32, (String, TranscriptReader)>,
}

#[cfg_attr(not(test), expect(dead_code, reason = "the context view reads the sessions, task 012"))]
impl Sessions {
    pub(crate) fn read(&mut self, claudes: &[Claude]) -> HashMap<u32, Breakdown> {
        self.readers.retain(|pid, _| claudes.iter().any(|claude| claude.pid == *pid));
        claudes.iter().filter_map(|claude| Some((claude.pid, self.breakdown(claude)?))).collect()
    }

    fn breakdown(&mut self, claude: &Claude) -> Option<Breakdown> {
        let session = session::session(&claude.config, claude.pid)?;
        if self.readers.get(&claude.pid).is_none_or(|(id, _)| *id != session.id) {
            self.readers.remove(&claude.pid);
            let path = session::transcript(&claude.config, &session)?;
            self.readers.insert(claude.pid, (session.id, TranscriptReader::new(path)));
        }
        let (_, reader) = self.readers.get_mut(&claude.pid)?;
        let breakdown = reader.update();
        if breakdown.is_none() {
            self.readers.remove(&claude.pid);
        }
        breakdown
    }
}

#[cfg(test)]
mod tests;
