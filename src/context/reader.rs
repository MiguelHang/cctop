use std::fs::{self, File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use super::breakdown::{Accumulator, Breakdown};
use super::record::parse;

#[derive(Debug)]
pub(crate) struct TranscriptReader {
    path: PathBuf,
    identity: Option<(u64, u64)>,
    offset: u64,
    partial: Vec<u8>,
    accumulator: Accumulator,
}

impl TranscriptReader {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path, identity: None, offset: 0, partial: Vec::new(), accumulator: Accumulator::default() }
    }

    pub(crate) fn update(&mut self) -> Option<Breakdown> {
        let metadata = fs::metadata(&self.path).ok()?;
        let identity = Some((metadata.dev(), metadata.ino()));
        if identity != self.identity || metadata.len() < self.offset {
            *self = Self { identity, ..Self::new(std::mem::take(&mut self.path)) };
        }
        if metadata.len() > self.offset {
            self.read_tail(&metadata)?;
        }
        Some(self.accumulator.breakdown())
    }

    fn read_tail(&mut self, metadata: &Metadata) -> Option<()> {
        let mut file = File::open(&self.path).ok()?;
        file.seek(SeekFrom::Start(self.offset)).ok()?;
        self.consume(file.take(metadata.len() - self.offset))
    }

    fn consume(&mut self, mut source: impl Read) -> Option<()> {
        let mut tail = Vec::new();
        source.read_to_end(&mut tail).ok()?;
        self.offset = self.offset.saturating_add(u64::try_from(tail.len()).unwrap_or(0));
        let mut bytes = std::mem::take(&mut self.partial);
        bytes.append(&mut tail);
        let complete = bytes.iter().rposition(|&byte| byte == b'\n').map_or(0, |end| end + 1);
        self.partial = bytes.split_off(complete);
        for line in bytes.split(|&byte| byte == b'\n').filter(|line| !line.is_empty()) {
            self.accumulator.push(parse(&String::from_utf8_lossy(line)));
        }
        Some(())
    }
}

#[cfg(test)]
mod tests;
