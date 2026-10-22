use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Record {
    Instructions(Vec<Entry>),
    Skills(usize),
    SystemPrompt(usize),
    Delta(Listing, Delta),
    CompactBoundary,
    User(Vec<UserBlock>),
    Assistant(Reply),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) chars: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Listing {
    Tools,
    Mcp,
    Agents,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Delta {
    pub(crate) added: Vec<Entry>,
    pub(crate) removed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UserBlock {
    Prompt(usize),
    ToolResult { id: String, chars: usize },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) text: usize,
    pub(crate) tools: Vec<ToolUse>,
    pub(crate) usage: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolUse {
    pub(crate) id: String,
    pub(crate) tool: Tool,
    pub(crate) chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tool {
    File(String),
    Command(String),
    Search(String),
    Subagent,
    Other,
}

pub(crate) fn parse(line: &str) -> Record {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Record::Other;
    };
    if value["isSidechain"].as_bool() == Some(true) {
        return Record::Other;
    }
    match value["type"].as_str() {
        Some("attachment") => attachment(&value["attachment"]),
        Some("system") if value["subtype"].as_str() == Some("compact_boundary") => Record::CompactBoundary,
        Some("user") => user(&value["message"]["content"]),
        Some("assistant") => assistant(&value["message"]),
        _ => Record::Other,
    }
}

fn attachment(value: &Value) -> Record {
    match value["type"].as_str() {
        Some("instructions") if value["files"].is_array() => Record::Instructions(instruction_files(&value["files"])),
        Some("skill_listing") => Record::Skills(chars(&value["content"])),
        Some("prompt_snapshot") => Record::SystemPrompt(strings(&value["systemPrompt"]).map(char_count).sum()),
        Some("deferred_tools_delta") => Record::Delta(Listing::Tools, delta(value, "addedNames", None, "removedNames")),
        Some("mcp_instructions_delta") => {
            Record::Delta(Listing::Mcp, delta(value, "addedNames", Some("addedBlocks"), "removedNames"))
        }
        Some("agent_listing_delta") => {
            Record::Delta(Listing::Agents, delta(value, "addedTypes", Some("addedLines"), "removedTypes"))
        }
        _ => Record::Other,
    }
}

fn instruction_files(files: &Value) -> Vec<Entry> {
    items(files)
        .filter_map(|file| Some(Entry { name: file["path"].as_str()?.to_owned(), chars: chars(&file["content"]) }))
        .collect()
}

fn delta(value: &Value, names: &str, texts: Option<&str>, removed: &str) -> Delta {
    let texts: Vec<&str> = texts.map(|key| strings(&value[key]).collect()).unwrap_or_default();
    let added = strings(&value[names])
        .enumerate()
        .map(|(index, name)| Entry { name: name.to_owned(), chars: char_count(texts.get(index).unwrap_or(&name)) })
        .collect();
    Delta { added, removed: strings(&value[removed]).map(str::to_owned).collect() }
}

fn user(content: &Value) -> Record {
    if let Some(text) = content.as_str() {
        return Record::User(vec![UserBlock::Prompt(char_count(text))]);
    }
    Record::User(items(content).filter_map(user_block).collect())
}

fn user_block(block: &Value) -> Option<UserBlock> {
    match block["type"].as_str()? {
        "text" => Some(UserBlock::Prompt(chars(&block["text"]))),
        "tool_result" => Some(UserBlock::ToolResult {
            id: block["tool_use_id"].as_str()?.to_owned(),
            chars: text(&block["content"]),
        }),
        _ => None,
    }
}

fn assistant(message: &Value) -> Record {
    let mut reply = Reply::default();
    for block in items(&message["content"]) {
        match block["type"].as_str() {
            Some("text") => reply.text += chars(&block["text"]),
            Some("thinking") => reply.text += chars(&block["thinking"]),
            Some("tool_use") => reply.tools.extend(tool_use(block)),
            _ => {}
        }
    }
    if message["model"].as_str() != Some("<synthetic>") {
        reply.usage = usage(&message["usage"]);
    }
    Record::Assistant(reply)
}

fn tool_use(block: &Value) -> Option<ToolUse> {
    let input = &block["input"];
    let field = |key: &str| input[key].as_str().map(str::to_owned);
    let tool = match block["name"].as_str()? {
        "Read" | "Edit" | "Write" => field("file_path").map(Tool::File),
        "Bash" => field("command").map(|command| Tool::Command(without_cd(&command).to_owned())),
        "Grep" | "Glob" => field("pattern").map(Tool::Search),
        "Agent" | "Task" => Some(Tool::Subagent),
        _ => None,
    };
    let chars = if input.is_null() { 0 } else { char_count(&input.to_string()) };
    Some(ToolUse { id: block["id"].as_str()?.to_owned(), tool: tool.unwrap_or(Tool::Other), chars })
}

fn without_cd(command: &str) -> &str {
    let mut rest = command;
    while let Some(next) = after_cd(rest.trim_start()) {
        rest = next;
    }
    if rest.len() == command.len() { command } else { rest.trim_start() }
}

fn after_cd(command: &str) -> Option<&str> {
    let path = command.strip_prefix("cd ")?.trim_start();
    let tail = after_word(path)?.trim_start();
    let rest = tail.strip_prefix("&&").or_else(|| tail.strip_prefix(';'))?;
    (!rest.trim().is_empty()).then_some(rest)
}

fn after_word(text: &str) -> Option<&str> {
    match text.chars().next()? {
        quote @ ('"' | '\'') => text[1..].find(quote).map(|end| &text[end + 2..]),
        _ => text
            .find(|character: char| character.is_whitespace() || matches!(character, ';' | '&'))
            .map(|end| &text[end..]),
    }
}

fn usage(value: &Value) -> Option<u64> {
    let counts: Vec<u64> = ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"]
        .iter()
        .filter_map(|key| value[key].as_u64())
        .collect();
    (!counts.is_empty()).then(|| counts.iter().sum())
}

fn text(content: &Value) -> usize {
    match content.as_str() {
        Some(text) => char_count(text),
        None => items(content).map(|block| chars(&block["text"])).sum(),
    }
}

fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

fn strings(value: &Value) -> impl Iterator<Item = &str> {
    items(value).filter_map(Value::as_str)
}

fn chars(value: &Value) -> usize {
    value.as_str().map_or(0, char_count)
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests;
