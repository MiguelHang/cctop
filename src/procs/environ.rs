use std::path::PathBuf;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Env {
    pub(crate) pane: Option<u64>,
    pub(crate) server: Option<String>,
    pub(crate) claude_config_dir: Option<PathBuf>,
    pub(crate) action_of: Option<String>,
}

pub(crate) fn parse_environ(bytes: &[u8]) -> Env {
    let mut env = Env::default();
    let (mut plugin, mut action) = (None, false);
    for entry in bytes.split(|&byte| byte == 0) {
        let entry = String::from_utf8_lossy(entry);
        let Some((key, value)) = entry.split_once('=').filter(|(_, value)| !value.is_empty()) else { continue };
        match key {
            "CORNERCASE_PANE" => env.pane = value.parse().ok(),
            "CORNERCASE_SERVER" => env.server = Some(value.to_owned()),
            "CLAUDE_CONFIG_DIR" => env.claude_config_dir = Some(PathBuf::from(value)),
            "CORNERCASE_PLUGIN_ID" => plugin = Some(value.to_owned()),
            "CORNERCASE_CONTEXT" => action = true,
            _ => {}
        }
    }
    env.action_of = plugin.filter(|_| action);
    env
}

#[cfg(test)]
mod tests;
