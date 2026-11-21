//! `fa usage record|show`: tokens by ticket, head and agent type, from the transcript files of a session.
//!
//! The record command streams the transcript (JSON lines) and keeps numbers and names only: the four
//! token counts, the git branch (turned into a ticket id when the board knows it), the tier ("head" for
//! the main transcript, "agent" for a subagent file), the agent type, and the count of `Agent` and `Skill`
//! tool calls by name. It keeps no text of the conversation, no tool input except the agent type and the
//! skill name, no path, and no model name (a rule of the owner: the file has no model information).
//! A reply has several lines with one message id: the id counts one time (the largest value of each
//! count wins, because a line that streams can have a smaller output count).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const USAGE: &str = "fa usage record --transcript F [--subagents DIR] [--board DIR] [--head NAME] --out FILE\n\
  Count the tokens of one session (numbers and names only, no text, no model) into one small JSON file.\n\
  --subagents DIR  the subagent files of the session (agent-*.jsonl, with agent-*.meta.json next to each)\n\
  --board DIR      the board folder for branch -> ticket (default: PLANNING/board of the repo)\n\
  --head NAME      the label of the head (default: the first 8 characters of the session id)\n\
fa usage show [--dir D] [--from-refs]\n\
  Print one table: ticket | head | tier/agent | input | output | cache read | cache write, and the sums\n\
  by ticket and by head. --dir D: the folder of usage files (default: FA_USAGE_DIR or the temp folder, fa-usage).\n\
  --from-refs: run `git fetch <USAGE_REMOTE, default the private store louellet-git/fa-data> +refs/usage/*:refs/usage/*` and read the files of the refs/usage/* refs.";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl Tokens {
    fn add(&mut self, o: &Tokens) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
    }
    fn max_with(&mut self, o: &Tokens) {
        self.input = self.input.max(o.input);
        self.output = self.output.max(o.output);
        self.cache_read = self.cache_read.max(o.cache_read);
        self.cache_write = self.cache_write.max(o.cache_write);
    }
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub ticket: String,
    /// "head" (main transcript) or "agent" (subagent file).
    pub tier: String,
    /// Agent type for tier "agent", empty for tier "head".
    pub agent_type: String,
    pub messages: u64,
    #[serde(flatten)]
    pub tokens: Tokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub version: u32,
    pub session: String,
    pub head: String,
    /// Date (UTC) of the last line of the main transcript.
    pub date: String,
    pub rows: Vec<Row>,
    pub agent_calls: BTreeMap<String, u64>,
    pub skill_calls: BTreeMap<String, u64>,
}

// Only the fields that we need. Serde skips the rest (text, tool input, model, paths).
#[derive(Deserialize)]
struct Line {
    #[serde(rename = "gitBranch")]
    branch: Option<String>,
    timestamp: Option<String>,
    message: Option<Msg>,
}
#[derive(Deserialize)]
struct Msg {
    id: Option<String>,
    usage: Option<Usage>,
    content: Option<Vec<Block>>,
}
#[derive(Deserialize, Default)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}
#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: Option<String>,
    id: Option<String>,
    name: Option<String>,
    input: Option<ToolInput>,
}
#[derive(Deserialize)]
struct ToolInput {
    subagent_type: Option<String>,
    skill: Option<String>,
}

struct Acc {
    /// message id -> (branch, tier, agent type, tokens)
    msgs: HashMap<String, (String, bool, String, Tokens)>,
    seen_tools: std::collections::HashSet<String>,
    agent_calls: BTreeMap<String, u64>,
    skill_calls: BTreeMap<String, u64>,
    last_ts: String,
}

fn stream_file(path: &Path, is_agent: bool, agent_type: &str, acc: &mut Acc) {
    let Ok(f) = std::fs::File::open(path) else { return };
    let mut rd = BufReader::with_capacity(1 << 20, f);
    let mut buf = String::new();
    loop {
        buf.clear();
        match rd.read_line(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        // Cheap filter: only assistant lines carry usage. A tool result inside a user line holds the
        // text escaped (\"type\":...), so this exact form is the top-level field only.
        if !buf.contains("\"type\":\"assistant\"") {
            continue;
        }
        let Ok(l) = serde_json::from_str::<Line>(&buf) else { continue };
        if !is_agent {
            if let Some(ts) = &l.timestamp {
                if *ts > acc.last_ts {
                    acc.last_ts = ts.clone();
                }
            }
        }
        let Some(m) = l.message else { continue };
        if let Some(blocks) = &m.content {
            for b in blocks {
                if b.kind.as_deref() != Some("tool_use") {
                    continue;
                }
                let first = match &b.id {
                    Some(id) => acc.seen_tools.insert(id.clone()),
                    None => true,
                };
                if !first {
                    continue;
                }
                let inp = b.input.as_ref();
                match b.name.as_deref() {
                    Some("Agent") => {
                        let t = inp.and_then(|i| i.subagent_type.clone()).unwrap_or_else(|| "(none)".into());
                        *acc.agent_calls.entry(t).or_insert(0) += 1;
                    }
                    Some("Skill") => {
                        let t = inp.and_then(|i| i.skill.clone()).unwrap_or_else(|| "(none)".into());
                        *acc.skill_calls.entry(t).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }
        let (Some(id), Some(u)) = (m.id, m.usage) else { continue };
        let t = Tokens {
            input: u.input_tokens.unwrap_or(0),
            output: u.output_tokens.unwrap_or(0),
            cache_read: u.cache_read_input_tokens.unwrap_or(0),
            cache_write: u.cache_creation_input_tokens.unwrap_or(0),
        };
        let e = acc.msgs.entry(id).or_insert_with(|| {
            (l.branch.clone().unwrap_or_default(), is_agent, agent_type.to_string(), Tokens::default())
        });
        e.3.max_with(&t);
    }
}

const BUILTIN_AGENTS: [&str; 7] =
    ["general-purpose", "Explore", "Plan", "web-fetch", "claude-code-guide", "statusline-setup", "claude"];

/// The names that may go into a usage file: the agents and skills of the repo, and the built-in agent types.
/// Any other name (it can be private) is written as "(other)".
#[derive(Debug, Default)]
pub struct Names {
    agents: std::collections::HashSet<String>,
    skills: std::collections::HashSet<String>,
}

impl Names {
    /// Read `.claude/agents/<name>.md` and `.claude/skills/<name>/` of the repo.
    pub fn from_repo(repo: &Path) -> Names {
        let mut n = Names::default();
        if let Ok(rd) = std::fs::read_dir(repo.join(".claude").join("agents")) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "md") {
                    if let Some(st) = p.file_stem().and_then(|s| s.to_str()) {
                        n.agents.insert(st.to_string());
                    }
                }
            }
        }
        if let Ok(rd) = std::fs::read_dir(repo.join(".claude").join("skills")) {
            for e in rd.flatten() {
                if e.path().is_dir() {
                    if let Some(st) = e.file_name().to_str() {
                        n.skills.insert(st.to_string());
                    }
                }
            }
        }
        n
    }
    fn agent(&self, t: &str) -> String {
        if t.is_empty() || t == "(none)" || t == "(unknown)" || BUILTIN_AGENTS.contains(&t) || self.agents.contains(t) {
            t.to_string()
        } else {
            "(other)".into()
        }
    }
    fn skill(&self, t: &str) -> String {
        if t == "(none)" || self.skills.contains(t) {
            t.to_string()
        } else {
            "(other)".into()
        }
    }
}

/// Count one session. `branch_to_ticket` maps a git branch to a ticket id; a branch that the board does
/// not know gives "no ticket" (a branch name can be private). `names` limits the agent and skill names.
pub fn count(
    transcript: &Path,
    subagents: Option<&Path>,
    branch_to_ticket: &BTreeMap<String, String>,
    names: &Names,
    head: Option<&str>,
) -> Summary {
    let mut acc = Acc {
        msgs: HashMap::new(),
        seen_tools: Default::default(),
        agent_calls: BTreeMap::new(),
        skill_calls: BTreeMap::new(),
        last_ts: String::new(),
    };
    stream_file(transcript, false, "", &mut acc);
    if let Some(dir) = subagents {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "jsonl")).collect())
            .unwrap_or_default();
        files.sort();
        for f in files {
            let meta = f.with_extension("meta.json");
            let ty = std::fs::read_to_string(&meta)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("agentType").and_then(|x| x.as_str()).map(str::to_string))
                .unwrap_or_else(|| "(unknown)".into());
            stream_file(&f, true, &ty, &mut acc);
        }
    }
    let mut rows: BTreeMap<(String, bool, String), Row> = BTreeMap::new();
    for (branch, is_agent, ty, t) in acc.msgs.into_values() {
        let ticket = branch_to_ticket.get(&branch).cloned().unwrap_or_else(|| "no ticket".to_string());
        let ty = names.agent(&ty);
        let key = (ticket.clone(), is_agent, ty.clone());
        let r = rows.entry(key).or_insert_with(|| Row {
            ticket,
            tier: if is_agent { "agent" } else { "head" }.into(),
            agent_type: ty,
            messages: 0,
            tokens: Tokens::default(),
        });
        r.messages += 1;
        r.tokens.add(&t);
    }
    let stem = transcript.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let session: String = stem.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    Summary {
        version: 1,
        head: head.map(str::to_string).unwrap_or_else(|| session.clone()),
        session,
        date: acc.last_ts.chars().take(10).collect(),
        rows: rows.into_values().collect(),
        agent_calls: relabel(acc.agent_calls, |t| names.agent(t)),
        skill_calls: relabel(acc.skill_calls, |t| names.skill(t)),
    }
}

fn relabel(m: BTreeMap<String, u64>, f: impl Fn(&str) -> String) -> BTreeMap<String, u64> {
    let mut o = BTreeMap::new();
    for (k, v) in m {
        *o.entry(f(&k)).or_insert(0) += v;
    }
    o
}

/// Branch -> ticket id, from the `branch` field of the tickets in `board` and `board/done`.
pub fn branch_map(board: &Path) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for (dir, done) in [(board.to_path_buf(), false), (board.join("done"), true)] {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
            if p.extension().is_none_or(|x| x != "md") || !stem.starts_with("T-") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let t = crate::board_check::parse_ticket(stem, done, &text);
            let b = t.get("branch");
            if !b.is_empty() {
                m.entry(b.to_string()).or_insert_with(|| stem.to_string());
            }
        }
    }
    m
}

// ---------------------------------------------------------------- table

pub fn render_table(sums: &[Summary]) -> String {
    let mut lines: BTreeMap<(String, String, String), Tokens> = BTreeMap::new();
    let mut by_ticket: BTreeMap<String, Tokens> = BTreeMap::new();
    let mut by_head: BTreeMap<String, Tokens> = BTreeMap::new();
    for s in sums {
        for r in &s.rows {
            let who = if r.tier == "head" { "head".to_string() } else { format!("agent:{}", r.agent_type) };
            lines.entry((r.ticket.clone(), s.head.clone(), who)).or_default().add(&r.tokens);
            by_ticket.entry(r.ticket.clone()).or_default().add(&r.tokens);
            by_head.entry(s.head.clone()).or_default().add(&r.tokens);
        }
    }
    let mut out = String::new();
    let hdr = |a: &str, b: &str, c: &str| format!("{a:<44} {b:<12} {c:<28}");
    let nums = |t: &Tokens| format!("{:>12} {:>10} {:>14} {:>12}", t.input, t.output, t.cache_read, t.cache_write);
    out.push_str(&format!("{} {:>12} {:>10} {:>14} {:>12}\n", hdr("ticket", "head", "tier/agent"), "input", "output", "cache read", "cache write"));
    for ((tk, h, w), t) in &lines {
        out.push_str(&format!("{} {}\n", hdr(tk, h, w), nums(t)));
    }
    out.push_str("\nSums by ticket\n");
    for (k, t) in &by_ticket {
        out.push_str(&format!("{} {}\n", hdr(k, "", ""), nums(t)));
    }
    out.push_str("\nSums by head\n");
    for (k, t) in &by_head {
        out.push_str(&format!("{} {}\n", hdr("", k, ""), nums(t)));
    }
    let mut ag: BTreeMap<&str, u64> = BTreeMap::new();
    let mut sk: BTreeMap<&str, u64> = BTreeMap::new();
    for s in sums {
        for (k, v) in &s.agent_calls {
            *ag.entry(k).or_insert(0) += v;
        }
        for (k, v) in &s.skill_calls {
            *sk.entry(k).or_insert(0) += v;
        }
    }
    let fmt = |m: &BTreeMap<&str, u64>| m.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ");
    out.push_str(&format!("\nAgent starts: {}\nSkill calls: {}\n", fmt(&ag), fmt(&sk)));
    out
}

// ---------------------------------------------------------------- commands

fn load_dir(dir: &Path) -> Vec<Summary> {
    let mut v = vec![];
    let Ok(rd) = std::fs::read_dir(dir) else { return v };
    let mut paths: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    paths.sort();
    for p in paths {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(x) = serde_json::from_str::<Summary>(&s) {
                v.push(x);
            }
        }
    }
    v
}

fn load_refs(repo: &Path) -> Vec<Summary> {
    let git = |args: &[&str]| Command::new("git").arg("-C").arg(repo).args(args).output();
    let remote = std::env::var("USAGE_REMOTE").unwrap_or_else(|_| "https://github.com/louellet-git/fa-data".to_string());
    let _ = git(&["fetch", "--quiet", &remote, "+refs/usage/*:refs/usage/*"]);
    let Ok(o) = git(&["for-each-ref", "--format=%(refname)", "refs/usage/"]) else { return vec![] };
    let mut v = vec![];
    for r in String::from_utf8_lossy(&o.stdout).lines() {
        if let Ok(b) = git(&["show", &format!("{r}:usage.json")]) {
            if let Ok(s) = serde_json::from_slice::<Summary>(&b.stdout) {
                v.push(s);
            }
        }
    }
    v
}

pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    let sub = args.first().map(String::as_str);
    let mut transcript = None;
    let (mut subagents, mut board, mut head, mut out, mut dir) = (None, None, None, None, None);
    let mut from_refs = false;
    let mut i = 1;
    while i < args.len() {
        let opt = args[i].as_str();
        let mut val = || -> Result<String, String> {
            i += 1;
            args.get(i).cloned().ok_or_else(|| format!("{opt} needs a value\n{USAGE}"))
        };
        match opt {
            "--transcript" => transcript = Some(PathBuf::from(val()?)),
            "--subagents" => subagents = Some(PathBuf::from(val()?)),
            "--board" => board = Some(PathBuf::from(val()?)),
            "--head" => head = Some(val()?),
            "--out" => out = Some(PathBuf::from(val()?)),
            "--dir" => dir = Some(PathBuf::from(val()?)),
            "--from-refs" => from_refs = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(0);
            }
            other => return Err(format!("unknown option '{other}'\n{USAGE}")),
        }
        i += 1;
    }
    match sub {
        Some("record") => {
            let t = transcript.ok_or_else(|| format!("record needs --transcript\n{USAGE}"))?;
            let o = out.ok_or_else(|| format!("record needs --out\n{USAGE}"))?;
            let bm = branch_map(&board.unwrap_or_else(|| repo.join("PLANNING").join("board")));
            let s = count(&t, subagents.as_deref(), &bm, &Names::from_repo(repo), head.as_deref());
            let json = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
            if let Some(p) = o.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            let tmp = o.with_extension("json.tmp");
            std::fs::write(&tmp, json + "\n").map_err(|e| format!("write {}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, &o).map_err(|e| format!("rename: {e}"))?;
            Ok(0)
        }
        Some("show") => {
            let sums = if from_refs {
                load_refs(repo)
            } else {
                let d = dir
                    .or_else(|| std::env::var_os("FA_USAGE_DIR").map(PathBuf::from))
                    .unwrap_or_else(|| std::env::temp_dir().join("fa-usage"));
                load_dir(&d)
            };
            if sums.is_empty() {
                println!("no usage files found");
            } else {
                print!("{}", render_table(&sums));
            }
            Ok(0)
        }
        _ => Err(format!("usage needs a command: record or show\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: &str, branch: &str, ts: &str, n: [u64; 4], tool: &str) -> String {
        let [inp, out, cr, cw] = n;
        format!(
            r#"{{"type":"assistant","gitBranch":"{branch}","timestamp":"{ts}","message":{{"model":"claude-test-1","id":"{id}","content":[{tool}],"usage":{{"input_tokens":{inp},"cache_creation_input_tokens":{cw},"cache_read_input_tokens":{cr},"output_tokens":{out}}}}}}}"#
        )
    }

    fn fixture(dir: &Path) -> (PathBuf, PathBuf) {
        std::fs::create_dir_all(dir.join("subagents")).unwrap();
        let tool_a = r#"{"type":"tool_use","id":"tu1","name":"Agent","input":{"subagent_type":"rust-engineer","prompt":"SECRET PROMPT"}}"#;
        let tool_s = r#"{"type":"tool_use","id":"tu2","name":"Skill","input":{"skill":"release-checks"}}"#;
        let main = [
            // m1 has two lines (same id): counted one time; the second line has the larger output
            line("m1", "claude/usage-log", "2026-11-21T10:00:00Z", [2, 10, 100, 5], tool_a),
            line("m1", "claude/usage-log", "2026-11-21T10:00:01Z", [2, 30, 100, 5], tool_s),
            line("m2", "main", "2026-11-21T11:00:00Z", [1, 7, 50, 0], r#"{"type":"text","text":"hello"}"#),
            r#"{"type":"user","message":{"content":"{\"type\":\"assistant\"}"}}"#.to_string(),
            "not json at all".to_string(),
        ]
        .join("\n");
        let p = dir.join("abcdef1234567890.jsonl");
        std::fs::write(&p, main).unwrap();
        let sub = line("s1", "claude/usage-log", "2026-11-21T10:30:00Z", [3, 4, 20, 6], "");
        std::fs::write(dir.join("subagents").join("agent-x.jsonl"), sub).unwrap();
        std::fs::write(dir.join("subagents").join("agent-x.meta.json"), r#"{"agentType":"data-engineer","model":"x"}"#).unwrap();
        (p, dir.join("subagents"))
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa-usage-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn names_of(d: &Path) -> Names {
        std::fs::create_dir_all(d.join(".claude").join("agents")).unwrap();
        std::fs::create_dir_all(d.join(".claude").join("skills").join("release-checks")).unwrap();
        std::fs::write(d.join(".claude").join("agents").join("rust-engineer.md"), "x").unwrap();
        Names::from_repo(d)
    }

    #[test]
    fn private_names_become_other_and_unknown_branch_is_no_ticket() {
        let d = tmpdir("priv");
        let (p, sub) = fixture(&d);
        let names = names_of(&d);
        let mut bm = BTreeMap::new();
        bm.insert("claude/usage-log".to_string(), "T-20261121-usage-log".to_string());
        // a private branch, a private agent type and a private skill name
        let text = std::fs::read_to_string(&p).unwrap().replace("\"main\"", "\"claude/luc-private-thing\"")
            .replace("rust-engineer", "luc-secret-agent").replace("release-checks", "luc-secret-skill");
        std::fs::write(&p, text).unwrap();
        let s = count(&p, Some(&sub), &bm, &names, None);
        assert!(s.rows.iter().any(|r| r.ticket == "no ticket"));
        assert!(s.rows.iter().all(|r| r.ticket == "no ticket" || r.ticket.starts_with("T-")));
        assert_eq!(s.agent_calls.get("(other)"), Some(&1));
        assert_eq!(s.skill_calls.get("(other)"), Some(&1));
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("luc-"), "{json}");
        // a name of the repo and a built-in type stay
        assert_eq!(names.agent("rust-engineer"), "rust-engineer");
        assert_eq!(names.agent("Explore"), "Explore");
        assert_eq!(names.skill("release-checks"), "release-checks");
        assert_eq!(names.skill("other-skill"), "(other)");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn counts_a_repeated_id_one_time() {
        let d = tmpdir("count");
        let (p, sub) = fixture(&d);
        let mut bm = BTreeMap::new();
        bm.insert("claude/usage-log".to_string(), "T-20261121-usage-log".to_string());
        let s = count(&p, Some(&sub), &bm, &names_of(&d), None);
        assert_eq!(s.session, "abcdef12");
        assert_eq!(s.head, "abcdef12");
        assert_eq!(s.date, "2026-11-21");
        assert_eq!(s.rows.len(), 3);
        let find = |t: &str, tier: &str| s.rows.iter().find(|r| r.ticket == t && r.tier == tier).unwrap().clone();
        let h = find("T-20261121-usage-log", "head");
        assert_eq!((h.messages, h.tokens), (1, Tokens { input: 2, output: 30, cache_read: 100, cache_write: 5 }));
        let a = find("T-20261121-usage-log", "agent");
        assert_eq!(a.agent_type, "(other)");
        assert_eq!(a.tokens, Tokens { input: 3, output: 4, cache_read: 20, cache_write: 6 });
        let n = find("no ticket", "head");
        assert_eq!(n.tokens.total(), 58);
        assert_eq!(s.agent_calls.get("rust-engineer"), Some(&1));
        assert_eq!(s.skill_calls.get("release-checks"), Some(&1));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn output_has_no_model_and_no_text() {
        let d = tmpdir("leak");
        let (p, sub) = fixture(&d);
        let out = d.join("out.json");
        let args: Vec<String> = ["record", "--transcript", p.to_str().unwrap(), "--subagents", sub.to_str().unwrap(), "--board", d.to_str().unwrap(), "--out", out.to_str().unwrap()]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(command(&d, &args).unwrap(), 0);
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(!text.contains("claude-"), "{text}");
        assert!(!text.contains("model"), "{text}");
        assert!(!text.contains("SECRET"), "{text}");
        assert!(!text.contains("hello"), "{text}");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(v.get("model").is_none());
        // the branch name "claude/usage-log" is a branch, not a model id: it has no "claude-"
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn branch_map_reads_the_board_field() {
        let d = tmpdir("board");
        std::fs::create_dir_all(d.join("done")).unwrap();
        std::fs::write(d.join("T-1-a.md"), "---\nid: T-1-a\nbranch: claude/a\n---\nbody").unwrap();
        std::fs::write(d.join("done").join("T-0-b.md"), "---\nid: T-0-b\nbranch: claude/b\n---\n").unwrap();
        let m = branch_map(&d);
        assert_eq!(m.get("claude/a").map(String::as_str), Some("T-1-a"));
        assert_eq!(m.get("claude/b").map(String::as_str), Some("T-0-b"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn table_has_fixed_sums() {
        let d = tmpdir("table");
        let (p, sub) = fixture(&d);
        let mut s = count(&p, Some(&sub), &BTreeMap::new(), &names_of(&d), Some("technology"));
        s.session = "abcdef12".into();
        let t = render_table(&[s]);
        let l = t.lines().find(|l| l.starts_with("no ticket") && l.contains("agent:(other)")).unwrap();
        assert!(l.split_whitespace().collect::<Vec<_>>().ends_with(&["3", "4", "20", "6"]), "{l}");
        assert!(t.contains("Sums by head"));
        let head_sum = t.lines().skip_while(|l| !l.starts_with("Sums by head")).nth(1).unwrap();
        // head total: input 2+1+3, output 30+7+4, cache read 100+50+20, cache write 5+0+6
        assert!(head_sum.split_whitespace().collect::<Vec<_>>().ends_with(&["technology", "6", "41", "170", "11"]), "{head_sum}");
        assert!(t.contains("Agent starts: rust-engineer 1"));
        assert!(t.contains("Skill calls: release-checks 1"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
