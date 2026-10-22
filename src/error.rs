#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("`{}` failed: {stderr}", argv.join(" "))]
    Command { argv: Vec<String>, stderr: String },
    #[error("`{}` answered something unexpected: {output}", argv.join(" "))]
    Answer { argv: Vec<String>, output: String },
    #[error(
        "none of CORNERCASE_PANE, CORNERCASE_TAB, CORNERCASE_WORKSPACE or CORNERCASE_PROJECT is set: \
         `cctop open` is the cctop action of cornercase's menu"
    )]
    NotInCornercase,
    #[error("cornercase lists no pane to split for {0}")]
    NoPane(String),
    #[error("that process has ended")]
    Ended,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
