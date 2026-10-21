use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use rstest::rstest;

const LEAVES: &[&str] = &["error", "format", "paths", "test_util"];

const DOMAINS: &[&str] = &["cornercase", "procs", "usage", "context", "alerts", "config"];

const RULES: &[(&str, &[&str])] = &[
    ("cornercase", &[]),
    ("procs", &[]),
    ("context", &[]),
    ("config", &[]),
    ("usage", &["cornercase", "procs"]),
    ("alerts", &["cornercase", "procs", "usage", "config"]),
    ("ui", DOMAINS),
    ("app", &["cornercase", "procs", "usage", "context", "alerts", "config", "ui"]),
];

const DEPENDENCY_EXCEPTIONS: &[(&str, &str)] = &[];
const LONG_FILES: &[&str] = &[];
const MAX_LINES: usize = 1000;

struct Source {
    path: PathBuf,
    module: Vec<String>,
    text: String,
}

impl Source {
    fn new(path: PathBuf, text: String) -> Self {
        let module = module_path(&path);
        Self { path, module, text }
    }

    fn production_uses(&self) -> BTreeSet<String> {
        if !self.path.starts_with("src") {
            return BTreeSet::new();
        }
        used_modules(&self.module, &self.text)
    }

    fn top_module(&self) -> Option<&str> {
        if !self.path.starts_with("src") || self.path.starts_with("src/main") {
            return None;
        }
        self.module.first().map(String::as_str)
    }
}

fn module_path(path: &Path) -> Vec<String> {
    let mut parts: Vec<_> = path
        .strip_prefix("src")
        .or_else(|_| path.strip_prefix("tests"))
        .expect("source path is under src or tests")
        .with_extension("")
        .iter()
        .map(|part| part.to_str().expect("source path is UTF-8").to_owned())
        .collect();
    if parts.last().is_some_and(|part| matches!(part.as_str(), "mod" | "lib" | "main")) {
        parts.pop();
    }
    parts
}

fn sources() -> Vec<Source> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    for folder in ["src", "tests"] {
        collect_paths(&root.join(folder), &mut paths);
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("read Rust source");
            let path = path.strip_prefix(root).expect("source is under crate root").to_owned();
            Source::new(path, text)
        })
        .collect()
}

fn collect_paths(folder: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let kind = entry.file_type().expect("read source entry type");
        if kind.is_dir() {
            collect_paths(&entry.path(), paths);
        } else if kind.is_file() && entry.path().extension().is_some_and(|extension| extension == "rs") {
            paths.push(entry.path());
        }
    }
}

fn raw_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start;
    if matches!(bytes[cursor], b'b' | b'c') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;
    let hashes = cursor;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    let ending = format!("\"{}", "#".repeat(cursor - hashes));
    let body = cursor + 1;
    Some(source[body..].find(&ending).map_or(bytes.len(), |offset| body + offset + ending.len()))
}

fn char_literal_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        match bytes.get(cursor)? {
            b'x' => cursor += 3,
            b'u' if bytes.get(cursor + 1) == Some(&b'{') => {
                cursor += source[cursor..].find('}')? + 1;
            }
            _ => cursor += 1,
        }
    } else {
        cursor += source.get(cursor..)?.chars().next()?.len_utf8();
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn code_tokens(source: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < source.len() {
        if let Some(end) = skipped_end(source, cursor) {
            cursor = end;
            continue;
        }
        let (token, end) = next_token(source, cursor);
        tokens.push(token);
        cursor = end;
    }
    tokens
}

fn skipped_end(source: &str, cursor: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let rest = &bytes[cursor..];
    if rest.starts_with(b"//") {
        return Some(cursor + source[cursor..].find('\n').unwrap_or(bytes.len() - cursor));
    }
    if rest.starts_with(b"/*") {
        return Some(block_comment_end(bytes, cursor));
    }
    if let Some(end) = raw_string_end(source, cursor) {
        return Some(end);
    }
    if rest[0] == b'"' {
        return Some(string_end(bytes, cursor));
    }
    if rest[0] == b'\''
        && let Some(end) = char_literal_end(source, cursor)
    {
        return Some(end);
    }
    rest[0].is_ascii_whitespace().then_some(cursor + 1)
}

fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 2;
    let mut depth = 1;
    while cursor < bytes.len() && depth > 0 {
        if bytes[cursor..].starts_with(b"/*") {
            depth += 1;
            cursor += 2;
        } else if bytes[cursor..].starts_with(b"*/") {
            depth -= 1;
            cursor += 2;
        } else {
            cursor += 1;
        }
    }
    cursor
}

fn string_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        cursor += 1;
        if byte == b'\\' {
            cursor = (cursor + 1).min(bytes.len());
        } else if byte == b'"' {
            break;
        }
    }
    cursor
}

fn next_token(source: &str, start: usize) -> (&str, usize) {
    let bytes = source.as_bytes();
    if bytes[start..].starts_with(b"::") {
        return (&source[start..start + 2], start + 2);
    }
    let raw_identifier = bytes[start..].starts_with(b"r#");
    let name = start + usize::from(raw_identifier) * 2;
    let first = source[name..].chars().next().expect("character at cursor");
    let mut cursor = name + first.len_utf8();
    if first.is_alphanumeric() || first == '_' {
        cursor = identifier_end(source, cursor);
    }
    (&source[name..cursor], cursor)
}

fn identifier_end(source: &str, mut cursor: usize) -> usize {
    while let Some(next) = source[cursor..].chars().next() {
        if !next.is_alphanumeric() && next != '_' {
            break;
        }
        cursor += next.len_utf8();
    }
    cursor
}

fn block_end(tokens: &[&str], start: usize) -> usize {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match *token {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

fn item_end(tokens: &[&str], start: usize) -> usize {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match *token {
            "(" | "[" => depth += 1,
            ")" | "]" => depth -= 1,
            ";" if depth == 0 => return index + 1,
            "{" if depth == 0 => return block_end(tokens, index),
            _ => {}
        }
    }
    tokens.len()
}

fn resolve_path(tokens: &[&str], mut cursor: usize, mut path: Vec<String>, used: &mut BTreeSet<String>) -> usize {
    if tokens.get(cursor) == Some(&"{") {
        cursor += 1;
        while cursor < tokens.len() && tokens[cursor] != "}" {
            cursor = resolve_path(tokens, cursor, path.clone(), used);
            while cursor < tokens.len() && !matches!(tokens[cursor], "," | "}") {
                cursor += 1;
            }
            if tokens.get(cursor) == Some(&",") {
                cursor += 1;
            }
        }
        return cursor + 1;
    }
    match tokens.get(cursor).copied() {
        Some("crate") => path.clear(),
        Some("super") => {
            path.pop();
        }
        Some("self" | "*") => {}
        Some(name) => path.push(name.to_owned()),
        None => return cursor,
    }
    cursor += 1;
    if tokens.get(cursor) == Some(&"::") {
        return resolve_path(tokens, cursor + 1, path, used);
    }
    if let Some(module) = path.first() {
        used.insert(module.clone());
    }
    cursor
}

type Visit<'a> = dyn FnMut(&[&str], usize, &[String]) -> usize + 'a;

fn scan_scope(tokens: &[&str], module: &[String], visit: &mut Visit) {
    if module.iter().any(|part| matches!(part.as_str(), "tests" | "test_util")) {
        return;
    }
    let mut cursor = 0;
    while cursor < tokens.len() {
        if tokens[cursor..].starts_with(&["#", "[", "cfg", "(", "test", ")", "]"]) {
            cursor = item_end(tokens, cursor + 7);
        } else if tokens[cursor] == "mod" && tokens.get(cursor + 2) == Some(&"{") {
            let mut child = module.to_vec();
            child.push(tokens[cursor + 1].to_owned());
            let end = block_end(tokens, cursor + 2);
            scan_scope(&tokens[cursor + 3..end - 1], &child, visit);
            cursor = end;
        } else {
            cursor = visit(tokens, cursor, module);
        }
    }
}

fn used_modules(module: &[String], source: &str) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    scan_scope(&code_tokens(source), module, &mut |tokens, cursor, module| {
        if matches!(tokens[cursor], "crate" | "super" | "self") && tokens.get(cursor + 1) == Some(&"::") {
            resolve_path(tokens, cursor, module.to_vec(), &mut used)
        } else {
            cursor + 1
        }
    });
    used
}

fn allowed(module: &str) -> Option<Vec<&'static str>> {
    if LEAVES.contains(&module) {
        return Some(LEAVES.to_vec());
    }
    let (_, uses) = RULES.iter().find(|(name, _)| *name == module)?;
    Some(LEAVES.iter().chain(uses.iter()).copied().collect())
}

fn excepted(source: &Source, used: &str, exceptions: &[(&str, &str)]) -> bool {
    exceptions.iter().any(|(path, module)| source.path == Path::new(path) && *module == used)
}

fn source_failures(source: &Source, exceptions: &[(&str, &str)]) -> Vec<String> {
    let Some(module) = source.top_module() else { return Vec::new() };
    let Some(allowed) = allowed(module) else {
        return vec![format!(
            "{} is in module {module}, which is not in the table of AGENTS.md (Where code goes). Put the code in one of its modules, or add the module to the table and to RULES or LEAVES in tests/layers.rs in the same change.",
            source.path.display()
        )];
    };
    source
        .production_uses()
        .into_iter()
        .filter(|used| used != module && !allowed.contains(&used.as_str()) && !excepted(source, used, exceptions))
        .map(|used| format!(
            "{} (module {}) uses crate::{used}. {module} may use only {}: move what it needs out of {used}, or move the code that needs it there (see Where code goes in AGENTS.md). Do not add it to DEPENDENCY_EXCEPTIONS: that list only shrinks.",
            source.path.display(), source.module.join("::"), allowed.join(", ")
        ))
        .collect()
}

fn dependency_failures(sources: &[Source], exceptions: &[(&str, &str)]) -> Vec<String> {
    let mut failures: Vec<String> = sources.iter().flat_map(|source| source_failures(source, exceptions)).collect();
    for (path, used) in exceptions {
        if !sources.iter().any(|source| source.path == Path::new(path) && source.production_uses().contains(*used)) {
            failures.push(format!(
                "{path} no longer uses crate::{used}, or is gone: remove it from DEPENDENCY_EXCEPTIONS in tests/layers.rs."
            ));
        }
    }
    failures
}

fn size_failures(sources: &[Source], exceptions: &[&str]) -> Vec<String> {
    let mut failures = Vec::new();
    for source in sources {
        let lines = source.text.lines().count();
        if lines > MAX_LINES && !exceptions.iter().any(|exception| source.path == Path::new(exception)) {
            failures.push(format!(
                "{} has {lines} lines, over {MAX_LINES}. Split it by concern into child modules, and move its tests to {}/tests.rs. Do not add it to LONG_FILES: that list only shrinks.",
                source.path.display(), source.module.join("/")
            ));
        }
    }
    for exception in exceptions {
        match sources.iter().find(|source| source.path == Path::new(exception)) {
            Some(source) if source.text.lines().count() <= MAX_LINES => failures.push(format!(
                "{exception} has {} lines now: remove it from LONG_FILES in tests/layers.rs.",
                source.text.lines().count()
            )),
            None => failures.push(format!(
                "{exception} is gone: remove it from LONG_FILES in tests/layers.rs, or rename it if the file moved."
            )),
            Some(_) => {}
        }
    }
    failures
}

fn declared_modules(lib: &str) -> BTreeSet<String> {
    lib.lines()
        .filter_map(|line| line.strip_suffix(';')?.rsplit_once("mod ").map(|(_, name)| name.to_owned()))
        .collect()
}

fn assert_clean(failures: &[String]) {
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn production_modules_follow_the_dependency_rules() {
    assert_clean(&dependency_failures(&sources(), DEPENDENCY_EXCEPTIONS));
}

#[test]
fn no_file_is_over_1000_lines() {
    assert_clean(&size_failures(&sources(), LONG_FILES));
}

#[test]
fn lib_rs_declares_exactly_the_modules_of_the_table() {
    let lib = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).expect("read src/lib.rs");
    let table: BTreeSet<String> =
        LEAVES.iter().chain(RULES.iter().map(|(name, _)| name)).map(|name| (*name).to_owned()).collect();
    assert_eq!(declared_modules(&lib), table);
}

#[rstest]
#[case::procs_uses_app("src/procs/mod.rs", "use crate::app;", false)]
#[case::usage_uses_ui("src/usage/mod.rs", "use crate::ui;", false)]
#[case::usage_uses_procs("src/usage/mod.rs", "use crate::{procs::Sample, cornercase::Snapshot, format};", true)]
#[case::usage_uses_context("src/usage/tree.rs", "use crate::context::Breakdown;", false)]
#[case::cornercase_uses_procs("src/cornercase/mod.rs", "use crate::procs;", false)]
#[case::alerts_uses_config(
    "src/alerts/watch.rs",
    "use crate::{config, usage, procs, cornercase, error::Result};",
    true
)]
#[case::alerts_uses_ui("src/alerts/mod.rs", "use crate::ui;", false)]
#[case::ui_uses_domains("src/ui/mod.rs", "use crate::{usage, context, alerts, config, format};", true)]
#[case::ui_uses_app("src/ui/view.rs", "use super::super::app::App;", false)]
#[case::app_uses_everything(
    "src/app/mod.rs",
    "use crate::{ui, usage, procs, cornercase, context, alerts, config, paths};",
    true
)]
#[case::a_leaf_uses_a_leaf("src/format.rs", "use crate::error::Error;", true)]
#[case::a_leaf_uses_a_domain("src/paths.rs", "use crate::config;", false)]
#[case::main_uses_app("src/main.rs", "use crate::app;", true)]
#[case::lib_uses_app("src/lib.rs", "pub fn run_tui() { crate::app::run(); }", true)]
#[case::an_unknown_module("src/server/mod.rs", "", false)]
#[case::its_tests_use_anything("src/procs/tests.rs", "use crate::{app, ui};", true)]
#[case::test_code_inside_a_file("src/procs/mod.rs", "#[cfg(test)] mod tests { use crate::app; }", true)]
#[case::strings_and_comments("src/procs/mod.rs", "// crate::app\nlet x = \"crate::ui\";", true)]
fn the_dependency_rules(#[case] path: &str, #[case] text: &str, #[case] accepted: bool) {
    let source = Source::new(PathBuf::from(path), text.to_owned());
    assert_eq!(dependency_failures(&[source], &[]).is_empty(), accepted, "{path}: {text}");
}

#[test]
fn a_stale_dependency_exception_fails() {
    let source = Source::new(PathBuf::from("src/procs/mod.rs"), "use crate::app;".to_owned());
    assert_eq!(
        dependency_failures(std::slice::from_ref(&source), &[("src/procs/mod.rs", "app")]),
        Vec::<String>::new()
    );
    let clean = Source::new(PathBuf::from("src/procs/mod.rs"), String::new());
    assert!(dependency_failures(&[clean], &[("src/procs/mod.rs", "app")])[0].contains("no longer uses"));
}

#[rstest]
#[case::at_the_limit(1000, true)]
#[case::one_line_over(1001, false)]
fn the_size_limit(#[case] lines: usize, #[case] accepted: bool) {
    for path in ["src/procs/linux.rs", "tests/integration.rs"] {
        let source = Source::new(PathBuf::from(path), "x\n".repeat(lines));
        assert_eq!(size_failures(&[source], &[]).is_empty(), accepted, "{path}");
    }
}

#[test]
fn a_stale_long_file_fails() {
    let short = Source::new(PathBuf::from("src/procs/linux.rs"), "x\n".to_owned());
    assert!(size_failures(&[short], &["src/procs/linux.rs"])[0].contains("remove it from LONG_FILES"));
    assert!(size_failures(&[], &["src/gone.rs"])[0].contains("is gone"));
}

#[test]
fn lib_rs_module_declarations_are_read() {
    let lib = "pub mod error;\nmod format;\n\n#[cfg(test)]\nmod test_util;\npub use error::Result;\n";
    let expected: BTreeSet<String> = ["error", "format", "test_util"].map(str::to_owned).into();
    assert_eq!(declared_modules(lib), expected);
}
