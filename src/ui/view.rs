use super::dialog::Dialog;

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
    pub(crate) cpu_history: Vec<Option<f64>>,
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
    pub(crate) dialog: Option<Dialog>,
    pub(crate) graphs: Graphs,
    pub(crate) memory_threshold: u64,
    pub(crate) newer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Graphs {
    pub(crate) cpu: Vec<f64>,
    pub(crate) memory: Vec<u64>,
    pub(crate) points: usize,
    pub(crate) machine_memory: Option<u64>,
    pub(crate) row_points: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ContextView {
    pub(crate) agents: Vec<AgentRow>,
    pub(crate) marked: Option<usize>,
    pub(crate) title: String,
    pub(crate) detail: ContextDetail,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AgentRow {
    pub(crate) label: String,
    pub(crate) percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ContextDetail {
    #[default]
    NoAgent,
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
    pub(crate) total: Option<u64>,
    pub(crate) percent: Option<ContextPercent>,
    pub(crate) loaded: Vec<ContextPart>,
    pub(crate) groups: Vec<ContextGroup>,
    pub(crate) scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ContextPercent {
    pub(crate) used: f64,
    pub(crate) window: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ContextGroup {
    pub(crate) name: String,
    pub(crate) noun: String,
    pub(crate) tokens: u64,
    pub(crate) items: Vec<ContextPart>,
    pub(crate) more: usize,
    pub(crate) more_tokens: u64,
    pub(crate) open: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ContextPart {
    pub(crate) label: String,
    pub(crate) tokens: u64,
    pub(crate) path: bool,
}
