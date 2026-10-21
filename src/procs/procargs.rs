use std::iter;

use super::environ::{Env, parse_environ};

#[derive(Debug, Default, PartialEq)]
pub(crate) struct ProcArgs {
    pub(crate) args: Vec<String>,
    pub(crate) env: Env,
}

pub(crate) fn parse_procargs(bytes: &[u8]) -> ProcArgs {
    let Some((count, rest)) = bytes.split_first_chunk::<4>() else { return ProcArgs::default() };
    let count = usize::try_from(i32::from_ne_bytes(*count)).unwrap_or(0);
    let Some(path_end) = rest.iter().position(|&byte| byte == 0) else { return ProcArgs::default() };
    let padded = &rest[path_end..];
    let mut strings = &padded[padded.iter().position(|&byte| byte != 0).unwrap_or(padded.len())..];
    let args = iter::from_fn(|| take_string(&mut strings))
        .take(count)
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect();
    ProcArgs { args, env: parse_environ(strings) }
}

fn take_string<'a>(rest: &mut &'a [u8]) -> Option<&'a [u8]> {
    if rest.is_empty() {
        return None;
    }
    let (string, after) = match rest.iter().position(|&byte| byte == 0) {
        Some(end) => (&rest[..end], &rest[end + 1..]),
        None => (*rest, &[][..]),
    };
    *rest = after;
    Some(string)
}

#[cfg(test)]
mod tests;
