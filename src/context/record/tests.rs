use rstest::rstest;

use super::{Delta, Entry, Listing, Record, Reply, Tool, ToolUse, UserBlock, parse};

fn entry(name: &str, chars: usize) -> Entry {
    Entry { name: name.to_owned(), chars }
}

mod attachments {
    use super::*;

    #[test]
    fn instructions_list_each_file_with_its_path_and_size() {
        let line = r#"{"type":"attachment","attachment":{"type":"instructions","files":[{"path":"/repo/CLAUDE.md","type":"Project","content":"@AGENTS.md"},{"path":"/repo/AGENTS.md","content":"ñandú"},{"content":"no path"}]}}"#;
        assert_eq!(parse(line), Record::Instructions(vec![entry("/repo/CLAUDE.md", 10), entry("/repo/AGENTS.md", 5)]));
    }

    #[test]
    fn a_file_without_content_weighs_nothing() {
        let line = r#"{"type":"attachment","attachment":{"type":"instructions","files":[{"path":"/repo/CLAUDE.md"}]}}"#;
        assert_eq!(parse(line), Record::Instructions(vec![entry("/repo/CLAUDE.md", 0)]));
    }

    #[test]
    fn the_skill_listing_is_its_content() {
        let line = r#"{"type":"attachment","attachment":{"type":"skill_listing","content":"- a: b"}}"#;
        assert_eq!(parse(line), Record::Skills(6));
    }

    #[test]
    fn the_system_prompt_adds_up_its_pieces() {
        let line = r#"{"type":"attachment","attachment":{"type":"prompt_snapshot","systemPrompt":["abc","de",7]}}"#;
        assert_eq!(parse(line), Record::SystemPrompt(5));
    }

    #[rstest]
    #[case::deferred_tools_by_name(
        r#"{"type":"attachment","attachment":{"type":"deferred_tools_delta","addedNames":["WebFetch","Bash"],"removedNames":["Old"]}}"#,
        Listing::Tools,
        vec![entry("WebFetch", 8), entry("Bash", 4)],
        vec!["Old"],
    )]
    #[case::mcp_by_block(
        r#"{"type":"attachment","attachment":{"type":"mcp_instructions_delta","addedNames":["docs","git"],"addedBlocks":["- docs: here"]}}"#,
        Listing::Mcp,
        vec![entry("docs", 12), entry("git", 3)],
        vec![],
    )]
    #[case::agents_by_line(
        r#"{"type":"attachment","attachment":{"type":"agent_listing_delta","addedTypes":["Plan"],"addedLines":["- Plan: plans"],"removedTypes":["Explore"]}}"#,
        Listing::Agents,
        vec![entry("Plan", 13)],
        vec!["Explore"],
    )]
    fn deltas_carry_what_was_added_and_removed(
        #[case] line: &str,
        #[case] listing: Listing,
        #[case] added: Vec<Entry>,
        #[case] removed: Vec<&str>,
    ) {
        let removed = removed.into_iter().map(str::to_owned).collect();
        assert_eq!(parse(line), Record::Delta(listing, Delta { added, removed }));
    }
}

mod messages {
    use super::*;

    #[test]
    fn a_plain_user_message_is_a_prompt() {
        let line = r#"{"type":"user","isSidechain":false,"message":{"role":"user","content":"hello"}}"#;
        assert_eq!(parse(line), Record::User(vec![UserBlock::Prompt(5)]));
    }

    #[test]
    fn user_blocks_are_prompts_and_tool_results() {
        let line = r#"{"type":"user","message":{"content":[{"type":"text","text":"hi"},{"type":"tool_result","tool_use_id":"toolu_1","content":"abcd"},{"type":"tool_result","tool_use_id":"toolu_2","content":[{"type":"text","text":"ab"},{"type":"image"},{"type":"text","text":"c"}]},{"type":"tool_result","content":"no id"},{"type":"image"}]}}"#;
        let blocks = vec![
            UserBlock::Prompt(2),
            UserBlock::ToolResult { id: "toolu_1".to_owned(), chars: 4 },
            UserBlock::ToolResult { id: "toolu_2".to_owned(), chars: 3 },
        ];
        assert_eq!(parse(line), Record::User(blocks));
    }

    #[test]
    fn an_assistant_reply_has_its_text_tools_and_usage() {
        let line = r#"{"type":"assistant","message":{"model":"claude-opus-5-5","content":[{"type":"thinking","thinking":"hmm","signature":"x"},{"type":"text","text":"done"},{"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"/a"}}],"usage":{"input_tokens":3,"cache_creation_input_tokens":10,"cache_read_input_tokens":100,"output_tokens":50}}}"#;
        let read = ToolUse { id: "toolu_1".to_owned(), tool: Tool::File("/a".to_owned()), chars: 18 };
        assert_eq!(parse(line), Record::Assistant(Reply { text: 7, tools: vec![read], usage: Some(113) }));
    }

    #[test]
    fn a_reply_without_usage_has_no_total() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"a"}]}}"#;
        assert_eq!(parse(line), Record::Assistant(Reply { text: 1, tools: vec![], usage: None }));
    }

    #[test]
    fn missing_usage_fields_count_as_zero() {
        let line = r#"{"type":"assistant","message":{"content":[],"usage":{"cache_read_input_tokens":40}}}"#;
        assert_eq!(parse(line), Record::Assistant(Reply { usage: Some(40), ..Reply::default() }));
    }

    #[test]
    fn a_synthetic_reply_gives_no_usage() {
        let line = r#"{"type":"assistant","message":{"model":"<synthetic>","content":[],"usage":{"input_tokens":0}}}"#;
        assert_eq!(parse(line), Record::Assistant(Reply::default()));
    }

    #[test]
    fn a_compact_boundary_is_recognised() {
        assert_eq!(parse(r#"{"type":"system","subtype":"compact_boundary"}"#), Record::CompactBoundary);
    }
}

mod tools {
    use super::*;

    fn tool_of(name: &str, input: &str) -> Option<Tool> {
        let line = format!(
            r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"t","name":"{name}","input":{input}}}]}}}}"#
        );
        match parse(&line) {
            Record::Assistant(reply) => reply.tools.into_iter().next().map(|tool_use| tool_use.tool),
            other => panic!("not a reply: {other:?}"),
        }
    }

    #[rstest]
    #[case::read("Read", r#"{"file_path":"/a.rs"}"#, Tool::File("/a.rs".to_owned()))]
    #[case::edit("Edit", r#"{"file_path":"/a.rs","old_string":"x"}"#, Tool::File("/a.rs".to_owned()))]
    #[case::write("Write", r#"{"file_path":"/b.rs","content":"y"}"#, Tool::File("/b.rs".to_owned()))]
    #[case::bash("Bash", r#"{"command":"ls -la"}"#, Tool::Command("ls -la".to_owned()))]
    #[case::grep("Grep", r#"{"pattern":"fn main","path":"/src"}"#, Tool::Search("fn main".to_owned()))]
    #[case::glob("Glob", r#"{"pattern":"**/*.rs"}"#, Tool::Search("**/*.rs".to_owned()))]
    #[case::agent("Agent", r#"{"prompt":"go"}"#, Tool::Subagent)]
    #[case::task("Task", r#"{"prompt":"go"}"#, Tool::Subagent)]
    #[case::read_without_a_path("Read", "{}", Tool::Other)]
    #[case::bash_with_a_wrong_type("Bash", r#"{"command":42}"#, Tool::Other)]
    #[case::anything_else("TodoWrite", r#"{"todos":[]}"#, Tool::Other)]
    fn each_tool_is_keyed_by_its_input(#[case] name: &str, #[case] input: &str, #[case] tool: Tool) {
        assert_eq!(tool_of(name, input), Some(tool));
    }

    #[rstest]
    #[case::semicolon("cd /x/y; git diff", "git diff")]
    #[case::and("cd /x/y && cargo test --locked", "cargo test --locked")]
    #[case::without_spaces("cd /x/y&&ls", "ls")]
    #[case::quoted_path(r#"cd "/a b/c" && git status"#, "git status")]
    #[case::single_quoted_path("cd '/a b'; ls", "ls")]
    #[case::twice("cd /x && cd y; make", "make")]
    #[case::leading_spaces("  cd /x; ls", "ls")]
    #[case::only_a_cd("cd /x/y", "cd /x/y")]
    #[case::nothing_after_it("cd /x/y && ", "cd /x/y && ")]
    #[case::an_unclosed_quote(r#"cd "/x && ls"#, r#"cd "/x && ls"#)]
    #[case::a_pipe_is_kept("cd /x | ls", "cd /x | ls")]
    #[case::not_a_cd("cdk deploy; ls", "cdk deploy; ls")]
    #[case::cd_later("ls; cd /x; ls", "ls; cd /x; ls")]
    fn a_leading_cd_is_left_out_of_the_command(#[case] command: &str, #[case] expected: &str) {
        let input = serde_json::json!({ "command": command }).to_string();
        assert_eq!(tool_of("Bash", &input), Some(Tool::Command(expected.to_owned())));
    }

    #[test]
    fn a_tool_use_without_an_id_is_dropped() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{}}]}}"#;
        assert_eq!(parse(line), Record::Assistant(Reply::default()));
    }
}

mod tolerance {
    use super::*;

    #[rstest]
    #[case::empty("")]
    #[case::invalid_json("not json")]
    #[case::truncated(r#"{"type":"user","message":{"content":"hel"#)]
    #[case::not_an_object("[1,2,3]")]
    #[case::no_type(r#"{"message":{"content":"x"}}"#)]
    #[case::unknown_type(r#"{"type":"mystery"}"#)]
    #[case::ignored_type(r#"{"type":"file-history-snapshot","snapshot":{}}"#)]
    #[case::other_system(r#"{"type":"system","subtype":"stop_hook_summary"}"#)]
    #[case::other_attachment(r#"{"type":"attachment","attachment":{"type":"hook_success"}}"#)]
    #[case::attachment_without_body(r#"{"type":"attachment"}"#)]
    #[case::instructions_files_is_a_string(r#"{"type":"attachment","attachment":{"type":"instructions","files":"x"}}"#)]
    #[case::sidechain_prompt(r#"{"type":"user","isSidechain":true,"message":{"content":"x"}}"#)]
    #[case::sidechain_boundary(r#"{"type":"system","subtype":"compact_boundary","isSidechain":true}"#)]
    fn odd_lines_are_other(#[case] line: &str) {
        assert_eq!(parse(line), Record::Other);
    }

    #[rstest]
    #[case::content_is_a_number(r#"{"type":"user","message":{"content":7}}"#, Record::User(vec![]))]
    #[case::no_message(r#"{"type":"assistant"}"#, Record::Assistant(Reply::default()))]
    fn wrong_shapes_give_empty_records(#[case] line: &str, #[case] record: Record) {
        assert_eq!(parse(line), record);
    }
}
