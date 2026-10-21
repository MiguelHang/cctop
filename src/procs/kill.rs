use std::io;

use libc::c_int;

use super::{Identity, Pid, identity};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) pid: Pid,
    pub(crate) started: u64,
    pub(crate) pgid: Pid,
}

pub(crate) trait Killer {
    fn term(&self, target: Target) -> Result<()>;
    fn term_group(&self, target: Target) -> Result<()>;
}

pub(crate) struct Signal;

impl Killer for Signal {
    fn term(&self, target: Target) -> Result<()> {
        let pid = checked(target.pid, 1)?;
        still_alive(target)?;
        send(pid)
    }

    fn term_group(&self, target: Target) -> Result<()> {
        let pgid = checked(target.pgid, 2)?;
        if own_group().is_none_or(|own| own == target.pgid) {
            return Err(refused(target.pgid));
        }
        still_alive(target)?;
        send(-pgid)
    }
}

pub(crate) fn own_group() -> Option<Pid> {
    Pid::try_from(unsafe { libc::getpgrp() }).ok().filter(|&pgid| pgid > 0)
}

fn still_alive(target: Target) -> Result<()> {
    let expected = Identity { started: target.started, pgid: target.pgid };
    if identity(target.pid) == Some(expected) { Ok(()) } else { Err(Error::Ended) }
}

fn checked(id: Pid, lowest: c_int) -> Result<c_int> {
    match c_int::try_from(id) {
        Ok(id) if id >= lowest => Ok(id),
        _ => Err(refused(id)),
    }
}

fn refused(id: Pid) -> Error {
    io::Error::new(io::ErrorKind::InvalidInput, format!("refusing to signal {id}")).into()
}

fn send(target: c_int) -> Result<()> {
    if unsafe { libc::kill(target, libc::SIGTERM) } == 0 { Ok(()) } else { Err(io::Error::last_os_error().into()) }
}

#[cfg(test)]
mod tests;
