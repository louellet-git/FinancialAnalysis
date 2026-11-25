//! `fa board check`: report the rule faults of the board (`PLANNING/board/README.md`).
//!
//! Read-only. It reads git only: the files of `origin/main` and the list of the remote branches as
//! the last `git fetch` left them. It does not fetch, it changes no file, and it always exits 0:
//! it is a report, it never stops work.
//!
//! The checks are pure functions of a `Board` (the parsed data). `gather` reads git into a `Board`.
//!
//! Check 7 rule (the repo merges by squash, so a merged branch is not an ancestor of `main`): the
//! work of a ticket is on `main` when its branch exists on the remote, the branch changed at least
//! one file since the merge base, and each file that it changed has the same content on
//! `origin/main` as on the branch tip. A ticket in that state, and not in `done/`, is a fault.
//! A rule that follows: a ticket in `done/` needs a `merged:` line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

pub const USAGE: &str = "fa board check [--json]\n\
  Report the rule faults of the board, for each head. It reads git only (origin/main and the remote\n\
  branches of the last `git fetch`), changes no file, and exits 0. Checks:\n\
  1 in-progress/review with no branch, or a branch the remote does not have\n\
  2 two or more tickets in progress on one branch\n\
  3 a ticket file on a remote branch (in progress there) and not on origin/main\n\
  4 note: a head with more than 3 tickets in progress\n\
  5 an open ticket not in EPICS.md, or a name in EPICS.md with no file\n\
  6 no head, order or acceptance; no story on a ticket created on or after 2026-11-21\n\
  7 the work is on main but the file is not in done/ (a squash merge: the branch exists on the remote\n\
    and each file that it changed has the same content on origin/main as on the branch tip);\n\
    also: a ticket in done/ with no `merged:` line";

const MAIN_REF: &str = "origin/main";
const STORY_FROM: &str = "2026-11-21";
/// A ticket in done/ needs a `merged:` line only if it was created on or after this date (older tickets are before the rule).
const MERGED_LINE_FROM: &str = "2026-11-20";
const MAX_IN_PROGRESS: usize = 3;

#[derive(Debug, Clone, Default)]
pub struct Ticket {
    pub id: String,
    pub done: bool,
    pub fields: BTreeMap<String, String>,
}

impl Ticket {
    pub fn get(&self, key: &str) -> &str {
        self.fields.get(key).map(String::as_str).unwrap_or("")
    }
    fn head(&self) -> String {
        let h = self.get("head");
        if h.is_empty() { "(none)".into() } else { h.into() }
    }
}

/// Parse the `---` block at the top of a ticket file (`key: value` lines).
pub fn parse_ticket(id: &str, done: bool, text: &str) -> Ticket {
    let mut t = Ticket { id: id.into(), done, fields: BTreeMap::new() };
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return t;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if !k.starts_with(' ') && !k.trim().is_empty() {
                t.fields.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    t
}

/// The ticket ids (`T-YYYYMMDD-slug`) that the text of EPICS.md names.
pub fn epic_ids(text: &str) -> BTreeSet<String> {
    let b = text.as_bytes();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i + 11 < b.len() {
        let boundary = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        if boundary && b[i] == b'T' && b[i + 1] == b'-' && b[i + 2..i + 10].iter().all(u8::is_ascii_digit) && b[i + 10] == b'-' {
            let mut j = i + 11;
            while j < b.len() && (b[j].is_ascii_lowercase() || b[j].is_ascii_digit() || b[j] == b'-') {
                j += 1;
            }
            let id = text[i..j].trim_end_matches('-');
            if id.len() > 11 {
                out.insert(id.to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// The data that the checks read.
#[derive(Debug, Default)]
pub struct Board {
    pub tickets: Vec<Ticket>,
    pub epics: String,
    /// Remote branch names without the `origin/` prefix.
    pub remote_branches: BTreeSet<String>,
    /// Tickets that are new files on a remote branch and not on main: (branch, ticket as on the branch).
    pub branch_only: Vec<(String, Ticket)>,
    /// Remote branches whose work is on main (check 7 rule).
    pub merged_branches: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    pub head: String,
    pub ticket: String,
    pub check: u8,
    pub note: bool,
    pub reason: String,
}

fn fault(t: &Ticket, check: u8, reason: impl Into<String>) -> Fault {
    Fault { head: t.head(), ticket: t.id.clone(), check, note: false, reason: reason.into() }
}

fn is_active(t: &Ticket) -> bool {
    matches!(t.get("status"), "in-progress" | "review")
}

pub fn check(b: &Board) -> Vec<Fault> {
    let mut out = Vec::new();
    let open: Vec<&Ticket> = b.tickets.iter().filter(|t| !t.done).collect();

    // 1
    for t in open.iter().filter(|t| is_active(t)) {
        let br = t.get("branch");
        if br.is_empty() {
            out.push(fault(t, 1, format!("{} with no branch", t.get("status"))));
        } else if !b.remote_branches.contains(br) {
            out.push(fault(t, 1, format!("branch {br} is not on the remote")));
        }
    }
    // 2
    let mut by_branch: BTreeMap<&str, Vec<&Ticket>> = BTreeMap::new();
    for t in open.iter().filter(|t| t.get("status") == "in-progress" && !t.get("branch").is_empty()) {
        by_branch.entry(t.get("branch")).or_default().push(t);
    }
    for (br, ts) in by_branch.iter().filter(|(_, ts)| ts.len() > 1) {
        for t in ts {
            out.push(fault(t, 2, format!("{} tickets in progress on branch {br}", ts.len())));
        }
    }
    // 3
    for (br, t) in &b.branch_only {
        if is_active(t) {
            out.push(fault(t, 3, format!("file is on {br} and not on {MAIN_REF}")));
        }
    }
    // 4
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for t in open.iter().filter(|t| t.get("status") == "in-progress") {
        *count.entry(t.head()).or_default() += 1;
    }
    for (head, n) in count.iter().filter(|(_, n)| **n > MAX_IN_PROGRESS) {
        out.push(Fault { head: head.clone(), ticket: "-".into(), check: 4, note: true, reason: format!("{n} tickets in progress (note, not a limit)") });
    }
    // 5
    let in_epics = epic_ids(&b.epics);
    for t in &open {
        if !in_epics.contains(&t.id) {
            out.push(fault(t, 5, "not in EPICS.md"));
        }
    }
    let known: BTreeSet<&str> = b.tickets.iter().map(|t| t.id.as_str()).collect();
    for id in in_epics.iter().filter(|id| !known.contains(id.as_str())) {
        out.push(Fault { head: "(none)".into(), ticket: id.clone(), check: 5, note: false, reason: "named in EPICS.md, no file".into() });
    }
    // 6
    for t in &open {
        let missing: Vec<&str> = ["head", "order", "acceptance"].into_iter().filter(|k| t.get(k).is_empty()).collect();
        if !missing.is_empty() {
            out.push(fault(t, 6, format!("no {}", missing.join(", "))));
        }
        if t.get("story").is_empty() && t.get("created") >= STORY_FROM {
            out.push(fault(t, 6, format!("created {} with no story", t.get("created"))));
        }
    }
    // 7
    for t in &open {
        let br = t.get("branch");
        if !br.is_empty() && b.merged_branches.contains(br) {
            out.push(fault(t, 7, format!("work of {br} is on {MAIN_REF}; file not in done/")));
        }
    }
    for t in b.tickets.iter().filter(|t| t.done && t.get("merged").is_empty() && t.get("created") >= MERGED_LINE_FROM) {
        out.push(fault(t, 7, "in done/ with no `merged:` line"));
    }
    out.sort_by(|a, b| (&a.head, a.check, &a.ticket).cmp(&(&b.head, b.check, &b.ticket)));
    out
}

pub fn render_table(faults: &[Fault]) -> String {
    if faults.is_empty() {
        return "board check: no fault\n".into();
    }
    let hw = faults.iter().map(|f| f.head.len()).max().unwrap_or(4).max(4);
    let tw = faults.iter().map(|f| f.ticket.len()).max().unwrap_or(6).max(6);
    let mut s = format!("{:<hw$}  {:<tw$}  chk  reason\n", "head", "ticket");
    for f in faults {
        s.push_str(&format!("{:<hw$}  {:<tw$}  {:>3}  {}\n", f.head, f.ticket, f.check, f.reason));
    }
    let n = faults.iter().filter(|f| !f.note).count();
    s.push_str(&format!("{n} fault(s), {} note(s)\n", faults.len() - n));
    s
}

pub fn render_json(faults: &[Fault]) -> String {
    let v: Vec<_> = faults
        .iter()
        .map(|f| serde_json::json!({"head": f.head, "ticket": f.ticket, "check": f.check, "note": f.note, "reason": f.reason}))
        .collect();
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

// ---- git (read only) ----

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output().map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// True when the work of `branch_ref` is on `main_ref` (the check 7 rule, see the top of the file).
pub fn work_is_on_main(repo: &Path, main_ref: &str, branch_ref: &str) -> bool {
    let Ok(base) = git(repo, &["merge-base", main_ref, branch_ref]) else { return false };
    // --no-renames: a rename lists the old and the new path, so a rename that main does not have reads "not merged".
    let Ok(changed) = git(repo, &["diff", "--no-renames", "--name-only", base.trim(), branch_ref]) else { return false };
    let files: Vec<&str> = changed.lines().filter(|l| !l.is_empty()).collect();
    // Limit: a branch that changed only board files (a ticket edit that main also has) proves no merged work,
    // so at least one changed file must be outside PLANNING/board/.
    if files.is_empty() || files.iter().all(|f| f.starts_with("PLANNING/board/")) {
        return false;
    }
    let mut args = vec!["diff", "--no-renames", "--name-only", main_ref, branch_ref, "--"];
    args.extend(files);
    matches!(git(repo, &args), Ok(d) if d.trim().is_empty())
}

pub fn gather(repo: &Path) -> Result<Board, String> {
    let listing = git(repo, &["ls-tree", "-r", "--name-only", MAIN_REF, "PLANNING/board/"])
        .map_err(|e| format!("cannot read {MAIN_REF} (run `git fetch origin` first): {e}"))?;
    let mut b = Board { epics: git(repo, &["show", &format!("{MAIN_REF}:PLANNING/board/EPICS.md")]).unwrap_or_default(), ..Default::default() };
    for path in listing.lines() {
        let Some(rest) = path.strip_prefix("PLANNING/board/") else { continue };
        let (done, name) = match rest.strip_prefix("done/") {
            Some(n) => (true, n),
            None => (false, rest),
        };
        if name.contains('/') || !name.starts_with("T-") || !name.ends_with(".md") {
            continue;
        }
        let text = git(repo, &["show", &format!("{MAIN_REF}:{path}")])?;
        b.tickets.push(parse_ticket(name.trim_end_matches(".md"), done, &text));
    }
    let refs = git(repo, &["for-each-ref", "--format=%(refname)", "refs/remotes/origin/"])?;
    for r in refs.lines() {
        if let Some(n) = r.strip_prefix("refs/remotes/origin/") {
            if n != "HEAD" && n != "main" {
                b.remote_branches.insert(n.to_string());
            }
        }
    }
    let ids: BTreeSet<String> = b.tickets.iter().map(|t| t.id.clone()).collect();
    for br in &b.remote_branches {
        let tip = format!("origin/{br}");
        let added = git(repo, &["diff", "--diff-filter=A", "--name-only", &format!("{MAIN_REF}...{tip}"), "--", "PLANNING/board/"]).unwrap_or_default();
        for path in added.lines() {
            let name = path.strip_prefix("PLANNING/board/").unwrap_or("");
            if name.contains('/') || !name.starts_with("T-") || !name.ends_with(".md") {
                continue;
            }
            let id = name.trim_end_matches(".md");
            if !ids.contains(id) {
                if let Ok(text) = git(repo, &["show", &format!("{tip}:{path}")]) {
                    b.branch_only.push((br.clone(), parse_ticket(id, false, &text)));
                }
            }
        }
    }
    let wanted: BTreeSet<&str> = b.tickets.iter().filter(|t| !t.done).map(|t| t.get("branch")).filter(|x| !x.is_empty()).collect();
    for br in wanted {
        if b.remote_branches.contains(br) && work_is_on_main(repo, MAIN_REF, &format!("origin/{br}")) {
            b.merged_branches.insert(br.to_string());
        }
    }
    Ok(b)
}

/// `fa board check`. Always returns 0.
pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        Some("check") => {}
        _ => return Err(format!("board needs a command: check\n{USAGE}")),
    }
    let mut json = false;
    for a in &args[1..] {
        match a.as_str() {
            "--json" => json = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(0);
            }
            other => return Err(format!("unknown option '{other}'\n{USAGE}")),
        }
    }
    match gather(repo) {
        Ok(b) => {
            let f = check(&b);
            if json { println!("{}", render_json(&f)) } else { print!("{}", render_table(&f)) }
        }
        Err(e) => println!("board check: {e}"),
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: &str, kv: &[(&str, &str)]) -> Ticket {
        let mut x = Ticket { id: id.into(), ..Default::default() };
        for (k, v) in kv {
            x.fields.insert((*k).into(), (*v).into());
        }
        x
    }
    /// A ticket with all fields right.
    fn good(id: &str, status: &str, branch: &str, head: &str) -> Ticket {
        t(id, &[("status", status), ("branch", branch), ("head", head), ("order", "1"), ("acceptance", "x"), ("story", "E1.S1"), ("created", "2026-11-21")])
    }
    fn board(ts: Vec<Ticket>, branches: &[&str]) -> Board {
        let epics = ts.iter().map(|t| format!("`{}`", t.id)).collect::<Vec<_>>().join(" ");
        Board { tickets: ts, epics, remote_branches: branches.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }
    fn checks(b: &Board, n: u8) -> Vec<String> {
        check(b).into_iter().filter(|f| f.check == n).map(|f| format!("{}|{}", f.ticket, f.reason)).collect()
    }

    #[test]
    fn parses_front_matter() {
        let p = parse_ticket("T-1", false, "---\nid: T-1\nacceptance: a: b\nbranch:\n---\nbody: no\n");
        assert_eq!(p.get("id"), "T-1");
        assert_eq!(p.get("acceptance"), "a: b");
        assert_eq!(p.get("branch"), "");
        assert_eq!(p.get("body"), "");
        assert!(parse_ticket("T-2", false, "no block").fields.is_empty());
    }

    #[test]
    fn finds_epic_ids() {
        let ids = epic_ids("| `T-20261121-a-b` → T-20261120-c, xT-20261120-no, `T-20261119-d-`.");
        let v: Vec<_> = ids.into_iter().collect();
        assert_eq!(v, ["T-20261119-d", "T-20261120-c", "T-20261121-a-b"]);
    }

    #[test]
    fn check1_branch_missing() {
        let b = board(vec![good("T-20261121-a", "in-progress", "", "quant"), good("T-20261121-b", "review", "gone", "quant"), good("T-20261121-c", "review", "here", "quant"), good("T-20261121-d", "backlog", "", "quant")], &["here"]);
        assert_eq!(checks(&b, 1), ["T-20261121-a|in-progress with no branch", "T-20261121-b|branch gone is not on the remote"]);
    }

    #[test]
    fn check2_two_on_one_branch() {
        let b = board(vec![good("T-20261121-a", "in-progress", "x", "quant"), good("T-20261121-b", "in-progress", "x", "quant"), good("T-20261121-c", "in-progress", "y", "quant")], &["x", "y"]);
        assert_eq!(checks(&b, 2).len(), 2);
        assert!(checks(&b, 2)[0].ends_with("2 tickets in progress on branch x"));
    }

    #[test]
    fn check3_ticket_only_on_branch() {
        let mut b = board(vec![], &["w"]);
        b.branch_only.push(("w".into(), good("T-20261121-n", "in-progress", "w", "staff")));
        b.branch_only.push(("w".into(), good("T-20261121-m", "backlog", "", "staff")));
        assert_eq!(checks(&b, 3), ["T-20261121-n|file is on w and not on origin/main"]);
    }

    #[test]
    fn check4_more_than_three_is_a_note() {
        let ts: Vec<_> = (0..4).map(|i| good(&format!("T-20261121-t{i}"), "in-progress", &format!("b{i}"), "quant")).collect();
        let b = board(ts, &["b0", "b1", "b2", "b3"]);
        let f: Vec<_> = check(&b).into_iter().filter(|f| f.check == 4).collect();
        assert_eq!(f.len(), 1);
        assert!(f[0].note && f[0].reason.starts_with("4 tickets"));
        let three = board((0..3).map(|i| good(&format!("T-20261121-t{i}"), "in-progress", &format!("b{i}"), "quant")).collect(), &["b0", "b1", "b2"]);
        assert!(checks(&three, 4).is_empty());
    }

    #[test]
    fn check5_epics_both_ways() {
        let mut b = board(vec![good("T-20261121-a", "backlog", "", "quant"), good("T-20261121-b", "backlog", "", "quant")], &[]);
        b.epics = "`T-20261121-a` `T-20261121-ghost`".into();
        assert_eq!(checks(&b, 5), ["T-20261121-ghost|named in EPICS.md, no file", "T-20261121-b|not in EPICS.md"]);
    }

    #[test]
    fn check6_fields_and_story() {
        let mut a = good("T-20261121-a", "backlog", "", "quant");
        a.fields.remove("order");
        a.fields.remove("acceptance");
        let mut s = good("T-20261121-s", "backlog", "", "quant");
        s.fields.remove("story");
        let mut old = good("T-20261120-o", "backlog", "", "quant");
        old.fields.remove("story");
        old.fields.insert("created".into(), "2026-11-20".into());
        let mut nohead = good("T-20261121-h", "backlog", "", "quant");
        nohead.fields.remove("head");
        let b = board(vec![a, s, old, nohead], &[]);
        assert_eq!(checks(&b, 6), ["T-20261121-h|no head", "T-20261121-a|no order, acceptance", "T-20261121-s|created 2026-11-21 with no story"]);
        assert_eq!(check(&b).iter().find(|f| f.ticket == "T-20261121-h").unwrap().head, "(none)");
    }

    #[test]
    fn check7_merged_not_done_and_done_without_merged() {
        let mut b = board(vec![good("T-20261121-a", "review", "m", "quant"), good("T-20261121-b", "review", "open", "quant")], &["m", "open"]);
        b.merged_branches.insert("m".into());
        let mut d = good("T-20261120-d", "done", "", "quant");
        d.done = true;
        let mut d2 = d.clone();
        d2.id = "T-20261120-e".into();
        d2.fields.insert("merged".into(), "pull request 1".into());
        let mut old = d.clone();
        old.id = "T-20261119-old".into();
        old.fields.insert("created".into(), "2026-11-19".into());
        b.tickets.push(old);
        b.tickets.push(d);
        b.tickets.push(d2);
        b.epics.push_str(" T-20261120-d T-20261120-e T-20261119-old");
        assert_eq!(checks(&b, 7), ["T-20261120-d|in done/ with no `merged:` line", "T-20261121-a|work of m is on origin/main; file not in done/"]);
    }

    #[test]
    fn table_groups_by_head_and_exit_text() {
        let b = board(vec![good("T-20261121-z", "in-progress", "", "quant"), good("T-20261121-a", "in-progress", "", "econ")], &[]);
        let s = render_table(&check(&b));
        let lines: Vec<_> = s.lines().collect();
        assert!(lines[1].starts_with("econ") && lines[2].starts_with("quant"));
        assert!(s.ends_with("2 fault(s), 0 note(s)\n"));
        assert_eq!(render_table(&[]), "board check: no fault\n");
        assert!(render_json(&check(&b)).contains("\"check\": 1"));
    }

    fn run(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"]).args(args).output().unwrap().status.success();
        assert!(ok, "git {args:?}");
    }

    #[test]
    fn squash_merge_rule_with_a_real_git_repo() {
        let dir = std::env::temp_dir().join(format!("fa-board-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        run(&dir, &["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "1\n").unwrap();
        run(&dir, &["add", "."]);
        run(&dir, &["commit", "-q", "-m", "base"]);
        run(&dir, &["checkout", "-q", "-b", "feat"]);
        std::fs::write(dir.join("a.txt"), "2\n").unwrap();
        std::fs::write(dir.join("b.txt"), "new\n").unwrap();
        run(&dir, &["add", "."]);
        run(&dir, &["commit", "-q", "-m", "work"]);
        run(&dir, &["checkout", "-q", "main"]);
        run(&dir, &["checkout", "-q", "-b", "same"]);
        run(&dir, &["checkout", "-q", "main"]);
        // not merged yet
        assert!(!work_is_on_main(&dir, "main", "feat"));
        // a squash: same content, new commit, feat is not an ancestor
        std::fs::write(dir.join("a.txt"), "2\n").unwrap();
        std::fs::write(dir.join("b.txt"), "new\n").unwrap();
        run(&dir, &["add", "."]);
        run(&dir, &["commit", "-q", "-m", "squash"]);
        assert!(work_is_on_main(&dir, "main", "feat"));
        // a branch with no change is not "merged"
        assert!(!work_is_on_main(&dir, "main", "same"));
        // main changes a file afterwards: the rule says not on main (conservative)
        std::fs::write(dir.join("a.txt"), "3\n").unwrap();
        run(&dir, &["commit", "-q", "-am", "later"]);
        assert!(!work_is_on_main(&dir, "main", "feat"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
