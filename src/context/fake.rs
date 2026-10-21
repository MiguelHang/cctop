use std::fs;
use std::path::Path;

pub(crate) fn start(config: &Path, pid: u32, id: &str, transcript: &str) {
    let sessions = config.join("sessions");
    let project = config.join("projects").join("-repo");
    fs::create_dir_all(&sessions).expect("create sessions folder");
    fs::create_dir_all(&project).expect("create project folder");
    let session = serde_json::json!({ "pid": pid, "sessionId": id, "cwd": "/repo" });
    fs::write(sessions.join(format!("{pid}.json")), session.to_string()).expect("write session file");
    fs::write(project.join(format!("{id}.jsonl")), transcript).expect("write transcript");
}
