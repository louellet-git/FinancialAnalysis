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

pub const USAGE: &str = "fa usage record [--transcript F] [--subagents DIR] [--board DIR] [--head NAME] [--out FILE]\n\
  Count the tokens of one session (numbers and names only, no text, no model) into one small JSON file.\n\
  --subagents DIR  the subagent files of the session (agent-*.jsonl, with agent-*.meta.json next to each)\n\
  --board DIR      the board folder for branch -> ticket (default: PLANNING/board of the repo)\n\
  --head NAME      the label of the head (default: the first 8 characters of the session id)\n\
fa usage show [--dir D]\n\
  Print one table: ticket | head | tier/agent | input | output | cache read | cache write, and the sums\n\
  by ticket and by head. --dir D: the folder of usage files (default: FA_USAGE_DIR or the temp folder, fa-usage).\n\
  On request: `fa usage record && fa usage show` (record finds the transcript of this machine and its subagents).\n\
  fa usage show --week [--since RFC3339] [--head NAME] [--root DIR] [--epics FILE]\n\
  Tokens and USD at list price for each ticket since the last weekly reset (Wednesday 16:00 UTC) or --since, from all\n\
  transcripts under --root (default ~/.claude/projects, sub-folders too). With --head: the share of the head\n\
  from the last table of PLANNING/board/EPICS.md (--epics), the used USD and the remaining USD.\n\
  Local only: nothing leaves the machine (owner, 2026-11-21: \"Let's not get overly complicated\").";

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
    cwd: Option<String>,
    timestamp: Option<String>,
    message: Option<Msg>,
}
#[derive(Deserialize)]
struct Msg {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
    content: Option<Vec<Block>>,
}
#[derive(Deserialize, Default)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_creation: Option<CacheCreation>,
}
#[derive(Deserialize, Default)]
struct CacheCreation {
    ephemeral_1h_input_tokens: Option<u64>,
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
    command: Option<String>,
    subagent_type: Option<String>,
    skill: Option<String>,
}

/// (branch, cwd branch), is agent, agent type, tokens, (cwd, timestamp)
type MsgRec = ((String, String), bool, String, Tokens, (String, String));

struct Acc {
    /// message id -> (branch, tier, agent type, tokens)
    msgs: HashMap<String, MsgRec>,
    seen_tools: std::collections::HashSet<String>,
    agent_calls: BTreeMap<String, u64>,
    skill_calls: BTreeMap<String, u64>,
    last_ts: String,
    /// folder -> branch of its git checkout (None: no checkout, or a detached HEAD)
    cwd_branch: HashMap<String, Option<String>>,
    /// (timestamp, worktree folder, branch) of each `git worktree add` in a Bash call
    worktrees: Vec<(String, PathBuf, String)>,
    /// week mode: only lines at or after this time (19 characters, "YYYY-MM-DDTHH:MM:SS")
    since: Option<String>,
    /// week mode: message id -> (model, 1-hour cache write tokens, the usage had no cache_creation split)
    detail: HashMap<String, (String, u64, bool)>,
}

/// Remove `.` and `..` from a path without the file system.
fn normalize(p: &Path) -> PathBuf {
    let mut o = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                o.pop();
            }
            c => o.push(c),
        }
    }
    o
}

/// The (folder, branch) pairs of the `git worktree add ... -b BRANCH` parts of a shell command.
// Limits (QA note): `git -C <dir> worktree add ...` and a path in a shell variable ("$WT") are not found; such
// a line only stays "no ticket". The timestamps of the transcript are compared as text, which is correct
// because each line has the same ISO form with "Z".
fn worktree_adds(cmd: &str, cwd: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    for part in cmd.split("git worktree add").skip(1) {
        let part = part.split(['\n', ';', '&', '|']).next().unwrap_or("");
        let (mut path, mut branch) = (None, None);
        let mut it = part.split_whitespace().map(|t| t.trim_matches(['"', '\'']));
        while let Some(t) = it.next() {
            if t == "-b" || t == "-B" {
                branch = it.next();
            } else if !t.starts_with('-') && path.is_none() {
                path = Some(t);
            }
        }
        if let (Some(p), Some(b)) = (path, branch) {
            out.push((normalize(&cwd.join(p)), b.to_string()));
        }
    }
    out
}

/// The branch that a folder has checked out. A folder that is missing or not a git checkout gives None.
fn git_branch(dir: &str) -> Option<String> {
    let o = std::process::Command::new("git").args(["-C", dir, "rev-parse", "--abbrev-ref", "HEAD"]).output().ok()?;
    let b = String::from_utf8(o.stdout).ok()?.trim().to_string();
    (o.status.success() && !b.is_empty() && b != "HEAD").then_some(b)
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
        if let Some(s) = &acc.since {
            if l.timestamp.as_deref().and_then(|t| t.get(..19)).unwrap_or("") < s.as_str() {
                continue;
            }
        }
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
                if let (Some("Bash"), Some(cmd)) = (b.name.as_deref(), inp.and_then(|i| i.command.as_deref())) {
                    if let (Some(c), true) = (&l.cwd, cmd.contains("git worktree add")) {
                        for (p, br) in worktree_adds(cmd, Path::new(c)) {
                            acc.worktrees.push((l.timestamp.clone().unwrap_or_default(), p, br));
                        }
                    }
                }
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
        // Both branches are kept: the line's own branch wins when the board knows it (it was right at the time
        // of the line); the branch that the cwd has NOW is the second choice (a reused worktree moves).
        let cwd_b = l.cwd.as_ref().and_then(|c| acc.cwd_branch.entry(c.clone()).or_insert_with(|| git_branch(c)).clone());
        let branch = (l.branch.clone().unwrap_or_default(), cwd_b.unwrap_or_default());
        if acc.since.is_some() {
            let w1 = u.cache_creation.as_ref().and_then(|c| c.ephemeral_1h_input_tokens).unwrap_or(0);
            let d = acc.detail.entry(id.clone()).or_insert_with(|| (m.model.clone().unwrap_or_default(), 0, false));
            d.1 = d.1.max(w1);
            d.2 = u.cache_creation.is_none();
        }
        let e = acc.msgs.entry(id).or_insert_with(|| {
            (branch, is_agent, agent_type.to_string(), Tokens::default(), (l.cwd.clone().unwrap_or_default(), l.timestamp.clone().unwrap_or_default()))
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

/// The ticket of a message: the line branch, then the branch of the cwd, then the branch of the latest worktree
/// that the session made for the cwd (at or before the line). "no ticket" when the board knows none of them.
fn ticket_of(worktrees: &[(String, PathBuf, String)], branch: &(String, String), cwd: &str, ts: &str, bm: &BTreeMap<String, String>) -> String {
    let made = worktrees.iter().filter(|(w, f, _)| w.as_str() <= ts && !cwd.is_empty() && normalize(Path::new(cwd)).starts_with(f)).max_by(|a, b| a.0.cmp(&b.0)).map(|w| w.2.clone());
    bm.get(&branch.0).or_else(|| bm.get(&branch.1)).or_else(|| made.as_ref().and_then(|b| bm.get(b))).cloned().unwrap_or_else(|| "no ticket".to_string())
}

/// Count one session.`branch_to_ticket` maps a git branch to a ticket id; a branch that the board does
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
        cwd_branch: HashMap::new(),
        worktrees: Vec::new(),
        since: None,
        detail: HashMap::new(),
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
    for (branch, is_agent, ty, t, (cwd, ts)) in acc.msgs.into_values() {
        // Third source: the latest worktree that the session made for this folder, at or before the line.
        let ticket = ticket_of(&acc.worktrees, &branch, &cwd, &ts, branch_to_ticket);
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

// ---------------------------------------------------------------- week

/// USD per million tokens. Source: the Anthropic model list, cached 2026-11-03 (list price, no discount).
struct Price {
    input: f64,
    output: f64,
    cache_read: f64,
    write_5m: f64,
    write_1h: f64,
}

/// The list price of a model, None when the table has no entry.
// The cache numbers of Haiku (0.1x, 1.25x and 2x of the input price) are NOT verified against the model list.
fn price(model: &str) -> Option<Price> {
    let (input, output, cache_read, write_5m, write_1h) = match model {
        "claude-opus-5-5" => (4.00, 20.00, 0.20, 5.00, 8.00),
        "claude-sonnet-5-5" => (2.00, 10.00, 0.10, 2.50, 4.00),
        "claude-fable-5-2" => (8.00, 40.00, 0.20, 10.00, 16.00),
        "claude-haiku-5-5" => (0.10, 0.50, 0.01, 0.125, 0.20),
        _ => return None,
    };
    Some(Price { input, output, cache_read, write_5m, write_1h })
}

fn cost(p: &Price, t: &Tokens, write_1h: u64) -> f64 {
    let w5 = t.cache_write.saturating_sub(write_1h);
    (t.input as f64 * p.input + t.output as f64 * p.output + t.cache_read as f64 * p.cache_read + w5 as f64 * p.write_5m + write_1h as f64 * p.write_1h) / 1e6
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Unix seconds of an RFC 3339 time ("2026-11-25T16:00:00Z", a fraction and a "+hh:mm" offset are allowed).
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || !(b[10] == b'T' || b[10] == b't' || b[10] == b' ') || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let n = |a: usize, z: usize| s.get(a..z)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(5, 7)?, n(8, 10)?, n(11, 13)?, n(14, 16)?, n(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    let mut rest = &s[19..];
    if let Some(r) = rest.strip_prefix('.') {
        rest = r.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let off = match rest {
        "Z" | "z" => 0,
        r if r.len() == 6 && (r.starts_with('+') || r.starts_with('-')) && r.as_bytes()[3] == b':' => {
            let v = r.get(1..3)?.parse::<i64>().ok()? * 3600 + r.get(4..6)?.parse::<i64>().ok()? * 60;
            if r.starts_with('-') { -v } else { v }
        }
        _ => return None,
    };
    Some(days_from_civil(y, mo, d) * 86400 + h * 3600 + mi * 60 + se - off)
}

pub fn format_utc(t: i64) -> String {
    let (y, m, d) = civil_from_days(t.div_euclid(86400));
    let s = t.rem_euclid(86400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s % 3600 / 60, s % 60)
}

/// The last weekly reset (Wednesday 16:00 UTC) at or before `now` (unix seconds).
pub fn last_reset(now: i64) -> i64 {
    let day = now.div_euclid(86400);
    let weekday = (day + 4).rem_euclid(7); // 0 = Sunday; 1970-01-01 was a Thursday
    let mut t = (day - (weekday - 3).rem_euclid(7)) * 86400 + 16 * 3600;
    if t > now {
        t -= 7 * 86400;
    }
    t
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct WeekRow {
    pub tokens: Tokens,
    pub usd: f64,
    /// Tokens (all four kinds) of messages whose model has no price.
    pub unpriced_tokens: u64,
    /// Cache write tokens of messages with no 5-minute/1-hour split (priced at the 5-minute rate).
    pub unsplit_write: u64,
}

impl WeekRow {
    fn add(&mut self, o: &WeekRow) {
        self.tokens.add(&o.tokens);
        self.usd += o.usd;
        self.unpriced_tokens += o.unpriced_tokens;
        self.unsplit_write += o.unsplit_write;
    }
}

#[derive(Debug, Default)]
pub struct Week {
    pub by_ticket: BTreeMap<String, WeekRow>,
    pub total: WeekRow,
    /// model -> messages, for each model without a price
    pub unpriced_models: BTreeMap<String, u64>,
}

/// Count the lines at or after `since` (a time with at least 19 characters, "YYYY-MM-DDTHH:MM:SS") of the files.
/// Each message id counts one time over all files (a subagent file can repeat the lines of its parent).
pub fn week(files: &[PathBuf], since: &str, bm: &BTreeMap<String, String>) -> Week {
    let mut acc = Acc {
        msgs: HashMap::new(),
        seen_tools: Default::default(),
        agent_calls: BTreeMap::new(),
        skill_calls: BTreeMap::new(),
        last_ts: String::new(),
        cwd_branch: HashMap::new(),
        worktrees: Vec::new(),
        since: Some(since.chars().take(19).collect()),
        detail: HashMap::new(),
    };
    for f in files {
        stream_file(f, false, "", &mut acc);
    }
    let mut w = Week::default();
    for (id, (branch, _, _, t, (cwd, ts))) in acc.msgs {
        let ticket = ticket_of(&acc.worktrees, &branch, &cwd, &ts, bm);
        let (model, w1h, unsplit) = acc.detail.remove(&id).unwrap_or_default();
        let mut r = WeekRow { tokens: t, unsplit_write: if unsplit { t.cache_write } else { 0 }, ..Default::default() };
        match price(&model) {
            Some(p) => r.usd = cost(&p, &t, w1h),
            None => {
                r.unpriced_tokens = t.total();
                *w.unpriced_models.entry(if model.is_empty() { "(none)".into() } else { model }).or_insert(0) += 1;
            }
        }
        w.by_ticket.entry(ticket).or_default().add(&r);
        w.total.add(&r);
    }
    w
}

/// All `*.jsonl` files under `root` (the sub-folders too: subagent files) that were changed at or after `since_secs`.
fn jsonl_files(root: &Path, since_secs: i64) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "jsonl") {
                let m = e.metadata().and_then(|m| m.modified()).ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64);
                if m.is_none_or(|m| m >= since_secs) {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

/// The share of a head in USD, from the last table of the epics file that has the columns "Head" and "Share in USD".
/// The cell can read "about 227". None when the table or the row is absent or has another form.
pub fn parse_share(text: &str, head: &str) -> Option<f64> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().rposition(|l| l.starts_with('|') && l.contains("Head") && l.contains("Share in USD"))?;
    let cells = |l: &str| l.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect::<Vec<_>>();
    let col = cells(lines[start]).iter().position(|c| c == "Share in USD")?;
    for l in lines[start + 1..].iter().take_while(|l| l.starts_with('|')) {
        let c = cells(l);
        if c.first().is_some_and(|h| h.eq_ignore_ascii_case(head)) {
            return c.get(col)?.split_whitespace().find_map(|w| w.replace(',', "").parse::<f64>().ok());
        }
    }
    None
}

pub fn render_week(w: &Week, since: &str, now: &str, head: Option<&str>, share: Option<f64>) -> String {
    let mut o = format!("Window: {since} to {now} (UTC)\n");
    let usd = |r: &WeekRow| {
        let mut s = format!("{:.2}", r.usd);
        if r.unpriced_tokens > 0 {
            s += &format!(" + (no price) {} tokens", r.unpriced_tokens);
        }
        s
    };
    o += &format!("{:<44} {:>12} {:>10} {:>14} {:>12}  {}\n", "ticket", "input", "output", "cache read", "cache write", "USD");
    let line = |k: &str, r: &WeekRow| format!("{:<44} {:>12} {:>10} {:>14} {:>12}  {}\n", k, r.tokens.input, r.tokens.output, r.tokens.cache_read, r.tokens.cache_write, usd(r));
    for (k, r) in &w.by_ticket {
        o += &line(k, r);
    }
    o += &line("TOTAL", &w.total);
    o += "USD at list price (Anthropic model list, cached 2026-11-03).\n";
    if w.total.unsplit_write > 0 {
        o += &format!("{} cache write tokens have no 5-minute/1-hour split: priced at the 5-minute rate.\n", w.total.unsplit_write);
    }
    if !w.unpriced_models.is_empty() {
        let m: Vec<String> = w.unpriced_models.iter().map(|(k, v)| format!("{k} ({v} messages)")).collect();
        o += &format!("(no price) for: {}. Their tokens are counted, no USD is added.\n", m.join(", "));
    }
    o += "Haiku cache prices are not verified.\n";
    if let Some(h) = head {
        match share {
            Some(s) => o += &format!("Head {h}: share {s:.2} USD, used {:.2} USD, remaining {:.2} USD\n", w.total.usd, s - w.total.usd),
            None => o += &format!("Head {h}: share not found (used {:.2} USD)\n", w.total.usd),
        }
    }
    o
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

/// The transcript of this machine when none is given: the newest `*.jsonl` under `~/.claude/projects/*/`
/// (the live session writes its file at each turn).
fn default_transcript() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for d in std::fs::read_dir(home.join(".claude").join("projects")).ok()?.flatten() {
        for f in std::fs::read_dir(d.path()).into_iter().flatten().flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let n = f.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().is_none_or(|(m, _)| n > *m) {
                    best = Some((n, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

fn default_dir() -> PathBuf {
    std::env::var_os("FA_USAGE_DIR").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("fa-usage"))
}

fn show_week(repo: &Path, since: Option<String>, head: Option<String>, root: Option<PathBuf>, epics: Option<PathBuf>, board: Option<PathBuf>) -> Result<i32, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs() as i64;
    let from = match since {
        Some(s) => parse_rfc3339(&s).ok_or_else(|| format!("--since needs an RFC 3339 time, for example 2026-11-25T16:00:00Z (got '{s}')"))?,
        None => last_reset(now),
    };
    let root = root.or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".claude").join("projects"))).ok_or("no HOME; give --root")?;
    let bm = branch_map(&board.unwrap_or_else(|| repo.join("PLANNING").join("board")));
    let w = week(&jsonl_files(&root, from), &format_utc(from), &bm);
    let share = head.as_deref().and_then(|h| {
        let text = std::fs::read_to_string(epics.unwrap_or_else(|| repo.join("PLANNING").join("board").join("EPICS.md"))).ok()?;
        parse_share(&text, h)
    });
    print!("{}", render_week(&w, &format_utc(from), &format_utc(now), head.as_deref(), share));
    Ok(0)
}

pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    let sub = args.first().map(String::as_str);
    let mut transcript = None;
    let (mut subagents, mut board, mut head, mut out, mut dir) = (None, None, None, None, None);
    let (mut week_mode, mut since, mut root, mut epics) = (false, None::<String>, None::<PathBuf>, None::<PathBuf>);
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
            "--week" => week_mode = true,
            "--since" => since = Some(val()?),
            "--root" => root = Some(PathBuf::from(val()?)),
            "--epics" => epics = Some(PathBuf::from(val()?)),
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
            let t = transcript.or_else(default_transcript).ok_or_else(|| format!("no transcript found; give --transcript\n{USAGE}"))?;
            if !t.is_file() {
                return Err(format!("transcript not found: {}", t.display()));
            }
            if subagents.is_none() {
                let d = t.with_extension("").join("subagents");
                if d.is_dir() {
                    subagents = Some(d);
                }
            }
            let short: String = t.file_stem().and_then(|x| x.to_str()).unwrap_or("session").chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
            let o = out.unwrap_or_else(|| default_dir().join(format!("{short}.json")));
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
            if week_mode || since.is_some() {
                return show_week(repo, since, head, root, epics, board);
            }
            let sums = load_dir(&dir.unwrap_or_else(default_dir));
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
    fn the_line_branch_wins_on_the_board_and_the_cwd_branch_fills_the_rest() {
        let d = tmpdir("cwd");
        let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&d).args(args).output().unwrap().status.success());
        git(&["init", "-q"]);
        git(&["checkout", "-q", "-b", "claude/cwd-ticket"]);
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x"]);
        let mk = |id: &str, cwd: &str, gb: &str| {
            let cwd = cwd.replace('\\', "/");
            format!(r#"{{"type":"assistant","cwd":"{cwd}","gitBranch":"{gb}","message":{{"id":"{id}","usage":{{"output_tokens":5}}}}}}"#)
        };
        let p = d.join("s.jsonl");
        let here = d.to_str().unwrap().to_string();
        let gone = d.join("missing").to_str().unwrap().to_string();
        // a: its own branch is not on the board, so the cwd branch gives the ticket;
        // b: the cwd is gone, so its own branch; c: its own branch is on the board and wins over the cwd branch.
        let text = [mk("a", &here, "claude/unknown"), mk("b", &gone, "main"), mk("c", &here, "main")].join("\n");
        std::fs::write(&p, text).unwrap();
        let mut bm = BTreeMap::new();
        bm.insert("claude/cwd-ticket".to_string(), "T-1-cwd".to_string());
        bm.insert("main".to_string(), "T-0-main".to_string());
        let s = count(&p, None, &bm, &Names::default(), None);
        let tk: Vec<_> = s.rows.iter().map(|r| (r.ticket.as_str(), r.tokens.output)).collect();
        assert_eq!(tk, [("T-0-main", 10), ("T-1-cwd", 5)]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_removed_worktree_gets_the_branch_that_the_session_made_for_it() {
        let d = tmpdir("wt");
        let add = |ts: &str, br: &str| {
            let cmd = format!("git worktree add -q ../wt-x -b {br} origin/main");
            format!(r#"{{"type":"assistant","cwd":"/tmp/a/main","timestamp":"{ts}","message":{{"content":[{{"type":"tool_use","name":"Bash","input":{{"command":"{cmd}"}}}}]}}}}"#)
        };
        let mk = |id: &str, ts: &str| {
            format!(r#"{{"type":"assistant","cwd":"/tmp/a/wt-x","gitBranch":"other","timestamp":"{ts}","message":{{"id":"{id}","usage":{{"output_tokens":5}}}}}}"#)
        };
        let text = [add("t1", "claude/x"), mk("m", "t2"), add("t3", "claude/y"), mk("n", "t4")].join("\n");
        let p = d.join("s.jsonl");
        std::fs::write(&p, text).unwrap();
        let bm = BTreeMap::from([("claude/x".to_string(), "T-x".to_string()), ("claude/y".to_string(), "T-y".to_string())]);
        let s = count(&p, None, &bm, &Names::default(), None);
        let tk: Vec<_> = s.rows.iter().map(|r| (r.ticket.as_str(), r.tokens.output)).collect();
        assert_eq!(tk, [("T-x", 5), ("T-y", 5)]);
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

    fn wline(id: &str, model: &str, ts: &str, n: [u64; 4], split: Option<(u64, u64)>) -> String {
        let [inp, out, cr, cw] = n;
        let cc = split.map(|(a, b)| format!(r#","cache_creation":{{"ephemeral_1h_input_tokens":{b},"ephemeral_5m_input_tokens":{a}}}"#)).unwrap_or_default();
        format!(
            r#"{{"type":"assistant","gitBranch":"claude/w","timestamp":"{ts}","message":{{"model":"{model}","id":"{id}","usage":{{"input_tokens":{inp},"cache_creation_input_tokens":{cw},"cache_read_input_tokens":{cr},"output_tokens":{out}{cc}}}}}}}"#
        )
    }

    #[test]
    fn last_reset_is_wednesday_16_utc() {
        let t = |s: &str| parse_rfc3339(s).unwrap();
        let r = |s: &str| format_utc(last_reset(t(s)));
        assert_eq!(r("2026-11-25T15:59:59Z"), "2026-11-18T16:00:00Z");
        assert_eq!(r("2026-11-25T16:00:00Z"), "2026-11-25T16:00:00Z");
        assert_eq!(r("2026-11-25T22:40:00Z"), "2026-11-25T16:00:00Z");
        assert_eq!(r("2026-11-29T09:00:00Z"), "2026-11-25T16:00:00Z"); // a Sunday
        assert_eq!(r("2026-12-02T15:59:59Z"), "2026-11-25T16:00:00Z");
        assert_eq!(r("2026-12-02T16:00:01Z"), "2026-12-02T16:00:00Z");
        assert_eq!(t("2026-11-25T18:00:00+02:00"), t("2026-11-25T16:00:00.5Z"));
        assert_eq!(parse_rfc3339("yesterday"), None);
    }

    #[test]
    fn week_prices_two_models_with_split_and_a_model_without_price() {
        let d = tmpdir("week");
        let text = [
            // opus: 1M in, 1M out, 1M read, 1M 5m write, 1M 1h write: 4 + 20 + 0.2 + 5 + 8 = 37.2
            wline("a", "claude-opus-5-5", "2026-11-25T17:00:00.123Z", [1_000_000, 1_000_000, 1_000_000, 2_000_000], Some((1_000_000, 1_000_000))),
            // the same id again (streamed line, smaller output): counted one time
            wline("a", "claude-opus-5-5", "2026-11-25T17:00:01Z", [1_000_000, 500_000, 1_000_000, 2_000_000], Some((1_000_000, 1_000_000))),
            // sonnet, no split: 1M writes at the 5m rate: 2 + 2.5 = 4.5
            wline("b", "claude-sonnet-5-5", "2026-11-25T18:00:00Z", [1_000_000, 0, 0, 1_000_000], None),
            // no price
            wline("c", "claude-unknown-9", "2026-11-25T18:30:00Z", [10, 20, 30, 40], None),
            // before the window
            wline("d", "claude-opus-5-5", "2026-11-25T15:59:59Z", [9_000_000, 0, 0, 0], None),
        ]
        .join("\n");
        let p = d.join("s.jsonl");
        std::fs::write(&p, text).unwrap();
        let bm = BTreeMap::from([("claude/w".to_string(), "T-w".to_string())]);
        let w = week(&[p], "2026-11-25T16:00:00Z", &bm);
        let r = &w.by_ticket["T-w"];
        assert_eq!(r.tokens, Tokens { input: 2_000_010, output: 1_000_020, cache_read: 1_000_030, cache_write: 3_000_040 });
        assert!((r.usd - 41.7).abs() < 1e-9, "{}", r.usd);
        assert_eq!(r.unpriced_tokens, 100);
        assert_eq!(r.unsplit_write, 1_000_040);
        assert_eq!(w.unpriced_models.get("claude-unknown-9"), Some(&1));
        let t = render_week(&w, "2026-11-25T16:00:00Z", "2026-11-25T20:00:00Z", Some("Technology"), Some(227.0));
        assert!(t.contains("(no price) 100 tokens"), "{t}");
        assert!(t.contains("priced at the 5-minute rate"), "{t}");
        assert!(t.contains("share 227.00 USD, used 41.70 USD, remaining 185.30 USD"), "{t}");
        let t = render_week(&w, "a", "b", Some("Technology"), None);
        assert!(t.contains("share not found"), "{t}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn share_is_read_from_the_last_table() {
        let text = "| Head | Share in USD |\n|---|---|\n| Technology | about 1 |\n\ntext\n\n**Table**\n\n| Head | Share of the limit | Share in USD | Cost |\n|---|---|---|---|\n| Technology | 12% | about 227 | 674.19 |\n| Staff | 2% | about 38 | 1 |\n\nafter\n";
        assert_eq!(parse_share(text, "Technology"), Some(227.0));
        assert_eq!(parse_share(text, "staff"), Some(38.0));
        assert_eq!(parse_share(text, "Nobody"), None);
        assert_eq!(parse_share("no table here", "Technology"), None);
        assert_eq!(parse_share("| Head | Other |\n|---|---|\n| Technology | 5 |\n", "Technology"), None);
    }
}
