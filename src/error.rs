use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("`{}` failed: {stderr}", argv.join(" "))]
    Command { argv: Vec<String>, stderr: String },
    #[error("`{}` answered something unexpected: {output}", argv.join(" "))]
    Answer { argv: Vec<String>, output: String },
    #[error("CORNERCASE_PLUGIN_STATE is not set: `cctop open` is the cctop action of cornercase's menu")]
    NotInCornercase,
    #[error("cornercase lists no cctop project with a workspace at {}", .0.display())]
    NoWorkspace(PathBuf),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
