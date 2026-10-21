#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ViewTab {
    #[default]
    Processes,
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Sort {
    Name,
    #[default]
    Memory,
    Cpu,
    Agent,
    Status,
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RowKind {
    Project,
    Workspace,
    Tab,
    Pane,
    #[default]
    Process,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Row {
    pub(crate) depth: u16,
    pub(crate) kind: RowKind,
    pub(crate) label: String,
    pub(crate) folded: Option<bool>,
    pub(crate) memory: Option<u64>,
    pub(crate) cpu: Option<f64>,
    pub(crate) agent: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) context: Option<f64>,
    pub(crate) hot: bool,
    pub(crate) estimated: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Header {
    pub(crate) memory: Option<u64>,
    pub(crate) cpu: Option<f64>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct View {
    pub(crate) tab: ViewTab,
    pub(crate) rows: Vec<Row>,
    pub(crate) scroll: usize,
    pub(crate) selected: Option<usize>,
    pub(crate) sort: Sort,
    pub(crate) header: Header,
    pub(crate) context: ContextView,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ContextView {
    #[default]
    NoPane,
    Reading {
        pane: String,
    },
    NoClaude {
        pane: String,
    },
    NoSession {
        pane: String,
        pid: u32,
    },
    Session(ContextSession),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ContextSession {
    pub(crate) pane: String,
    pub(crate) pid: u32,
    pub(crate) total: Option<u64>,
    pub(crate) percent: Option<ContextPercent>,
    pub(crate) fixed: Vec<ContextPart>,
    pub(crate) dynamic: Vec<ContextPart>,
    pub(crate) scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ContextPercent {
    pub(crate) used: f64,
    pub(crate) window: u64,
    pub(crate) cornercase: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ContextPart {
    pub(crate) label: String,
    pub(crate) tokens: u64,
    pub(crate) path: bool,
}
