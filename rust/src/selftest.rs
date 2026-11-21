//! `fa selftest golden`: stored commands with the SHA-256 of their expected stdout.
//!
//! One case file (default `rust/tests/golden/cases.toml`) lists the cases. Each case has the `fa` arguments, the input
//! files that the command reads (each with its SHA-256), a mask (lines that vary by design: timing lines; any other mask
//! needs a `why`) and the SHA-256 of the masked stdout. The masked expected text is stored next to the case file
//! (`<name>.txt`), so a difference can show the first lines that differ.
//!
//! The run never downloads: it runs only commands of a small allow list that read the cache, it refuses the flags that
//! fetch, and it points the proxy of the child to a local listener that writes down each request. A case whose command
//! asks for the network FAILS, and the message names the request.
//!
//! An input that is listed and is missing, or has another hash, makes the case SKIPPED (named, with the file): it is not
//! a pass. An input that the list misses can only make the case FAIL (the output changes), never pass.
//! Exit codes: 0 all pass or skipped, 1 a failure, 2 an error of the run, 64 usage.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

pub const USAGE: &str = "\
fa selftest golden [--case NAME] [--cases PATH] [--update REASON [--case NAME | --all]] [--record-inputs [--case NAME | --all]]
  Run the stored cases with this binary and compare the SHA-256 of the masked stdout with the stored hash.
  Default case file: rust/tests/golden/cases.toml of the repository (FA_REPO). The data cache is data/raw of it.
  Each case lists the input files that it reads, with their SHA-256 (paths relative to the repository root).
    - A listed input that is missing or has another hash: the case is SKIPPED (named, with the file). Never a pass.
    - An input that the list misses can only make the case FAIL (the output changes), never pass. When it happens, add
      the file to the list (`--record-inputs`).
    - The `store` commit of a case is information only (the commit of the data store at the last update).
  Exit 0: no case failed (the numbers passed / failed / skipped are printed; a run where no case ran says so loudly).
  Exit 1: a case failed (the first 20 lines that differ).
  --case NAME       run only this case
  --cases PATH      another case file (the expected texts are next to it)
  --update REASON   write the new expected text and the new input hashes of the named cases, with the reason and the
                    date. A reason is required. Needs --case NAME or --all.
  --record-inputs   run the case(s) and write the list of the files that the command opens, with their hashes, into the
                    case file. It uses strace (Linux, `strace` must be installed); no expected text changes.
                    Without strace, write the list by hand and say so in the case (`note`).
  The mask has two forms: `prefix` (a line that starts with it) and `contains` (a line that has it). A masked line is
  replaced by `<masked>`. A mask that is not a timing line (`timing = true`) needs a `why`. A case can list
  `must_contain` texts that must be in the output (for example the headline numbers).
  The runner never downloads: only commands of an allow list that read the cache (dcf drivers|attribute, options check,
  trade backtest, vol fit), no flag that fetches, and a case that asks for the network FAILS and names the request.
  Not in CI: the store is private.";

/// Commands (the first words) that read the cache only.
pub const ALLOWED: [&[&str]; 5] = [&["dcf", "drivers"], &["dcf", "attribute"], &["options", "check"], &["trade", "backtest"], &["vol", "fit"]];
/// Flags that make a command fetch.
const FETCH_FLAGS: [&str; 5] = ["--refresh", "--force", "--fetch", "--download", "--live"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MaskRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub timing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// One input file: the path relative to the repository root, and its SHA-256.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Input {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Case {
    pub name: String,
    pub args: Vec<String>,
    /// The commit of the data store at the last update. Information only.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub store: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_contain: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mask: Vec<MaskRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<Input>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CaseFile {
    #[serde(default, rename = "case")]
    pub cases: Vec<Case>,
}

// ---------------------------------------------------------------------------------------------
// SHA-256 (FIPS 180-4); the crate has no hash dependency
// ---------------------------------------------------------------------------------------------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be,
    0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa,
    0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85,
    0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f,
    0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn compress(h: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (i, c) in block.chunks_exact(4).enumerate() {
        w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let mut v = *h;
    for i in 0..64 {
        let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
        let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
        let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
        let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
        let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
        let t2 = s0.wrapping_add(maj);
        v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
    }
    for (a, b) in h.iter_mut().zip(v) {
        *a = a.wrapping_add(b);
    }
}

/// The SHA-256 of what `r` gives, as 64 lower-case hex digits (a stream: a large file is not held in memory).
pub fn sha256_reader(mut r: impl std::io::Read) -> std::io::Result<String> {
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut buf = vec![0u8; 1 << 16];
    let mut pending: Vec<u8> = Vec::with_capacity(128);
    let mut total: u64 = 0;
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        pending.extend_from_slice(&buf[..n]);
        let whole = pending.len() / 64 * 64;
        for block in pending[..whole].chunks_exact(64) {
            compress(&mut h, block);
        }
        pending.drain(..whole);
    }
    pending.push(0x80);
    while pending.len() % 64 != 56 {
        pending.push(0);
    }
    pending.extend_from_slice(&(total * 8).to_be_bytes());
    for block in pending.chunks_exact(64) {
        compress(&mut h, block);
    }
    Ok(h.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:08x}");
        s
    }))
}

/// The SHA-256 of `data` as 64 lower-case hex digits.
pub fn sha256_hex(data: &[u8]) -> String {
    sha256_reader(data).expect("a slice does not fail")
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    sha256_reader(std::fs::File::open(path)?)
}

// ---------------------------------------------------------------------------------------------
// Mask, allow list, diff, inputs
// ---------------------------------------------------------------------------------------------

/// The text with each masked line replaced by `<masked>`. The result ends with one newline.
pub fn apply_mask(text: &str, mask: &[MaskRule]) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let hit = mask.iter().any(|m| m.prefix.as_deref().is_some_and(|p| line.starts_with(p)) || m.contains.as_deref().is_some_and(|c| line.contains(c)));
        out.push_str(if hit { "<masked>" } else { line });
        out.push('\n');
    }
    out
}

/// A rule needs a text, and a reason unless it is a timing line.
fn check_mask(case: &Case) -> Result<(), String> {
    for m in &case.mask {
        if m.prefix.is_none() && m.contains.is_none() {
            return Err(format!("case {}: a mask rule needs `prefix` or `contains`", case.name));
        }
        if m.prefix.as_deref() == Some("") || m.contains.as_deref() == Some("") {
            return Err(format!("case {}: an empty mask text would mask each line", case.name));
        }
        if !m.timing && m.why.as_deref().is_none_or(|w| w.trim().is_empty()) {
            return Err(format!("case {}: a mask that is not a timing line needs a `why`", case.name));
        }
    }
    Ok(())
}

/// Err with the reason when the command can download or is not on the allow list.
pub fn check_allowed(args: &[String]) -> Result<(), String> {
    let ok = ALLOWED.iter().any(|a| args.len() >= a.len() && a.iter().zip(args).all(|(x, y)| x == y));
    if !ok {
        return Err(format!("`fa {}` is not on the allow list of commands that read the cache only", args.join(" ")));
    }
    if let Some(f) = args.iter().find(|a| FETCH_FLAGS.contains(&a.as_str())) {
        return Err(format!("the flag {f} can download; the golden runner never downloads"));
    }
    Ok(())
}

/// The first `limit` lines that differ: (line number from 1, expected, actual).
pub fn first_diffs(expected: &str, actual: &str, limit: usize) -> Vec<(usize, String, String)> {
    let (e, a): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), actual.lines().collect());
    (0..e.len().max(a.len()))
        .filter(|&i| e.get(i) != a.get(i))
        .take(limit)
        .map(|i| (i + 1, e.get(i).map_or("<no line>".to_string(), |s| s.to_string()), a.get(i).map_or("<no line>".to_string(), |s| s.to_string())))
        .collect()
}

/// The first listed input that is missing or has another hash: the reason, with the file.
pub fn input_problem(root: &Path, inputs: &[Input]) -> Option<String> {
    for i in inputs {
        match sha256_file(&root.join(&i.path)) {
            Err(_) => return Some(format!("input {} is missing", i.path)),
            Ok(h) if h != i.sha256 => return Some(format!("input {} has another hash", i.path)),
            Ok(_) => {}
        }
    }
    None
}

fn hash_inputs(root: &Path, paths: &[String]) -> Result<Vec<Input>, String> {
    paths.iter().map(|p| sha256_file(&root.join(p)).map(|h| Input { path: p.clone(), sha256: h }).map_err(|e| format!("input {p}: {e}"))).collect()
}

/// The files that a command opened for reading, from the text of `strace -f -e trace=openat`: paths under `root` that are
/// regular files, relative to `root`, sorted, once each. Folders, failed opens, `.git` and `rust/target` are left out.
pub fn paths_from_strace(trace: &str, root: &Path) -> Vec<String> {
    let mut found = std::collections::BTreeSet::new();
    for line in trace.lines() {
        let Some(at) = line.find("openat(") else { continue };
        let rest = &line[at + 7..];
        let Some(q1) = rest.find('"') else { continue };
        let Some(q2) = rest[q1 + 1..].find('"') else { continue };
        let path = &rest[q1 + 1..q1 + 1 + q2];
        let after = &rest[q1 + 2 + q2..];
        let ok = after.rsplit('=').next().and_then(|r| r.split_whitespace().next()).and_then(|n| n.parse::<i64>().ok()).is_some_and(|n| n >= 0);
        if !ok || after.contains("O_WRONLY") || after.contains("O_RDWR") || after.contains("O_DIRECTORY") {
            continue;
        }
        let abs = if Path::new(path).is_absolute() { PathBuf::from(path) } else { root.join(path) };
        let Ok(rel) = abs.strip_prefix(root) else { continue };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if rel_s.starts_with(".git/") || rel_s.starts_with("rust/target/") || !abs.is_file() {
            continue;
        }
        found.insert(rel_s);
    }
    found.into_iter().collect()
}

// ---------------------------------------------------------------------------------------------
// Environment of a run (the store and the child are replaceable in tests)
// ---------------------------------------------------------------------------------------------

/// What one child run gave.
#[derive(Debug, Default, Clone)]
pub struct RunOutput {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
    /// The network requests that the child tried (the first line of each, for example `CONNECT host:443 HTTP/1.1`).
    pub requests: Vec<String>,
}

pub type RunResult = Result<RunOutput, String>;
/// The files that a command opens, or the reason it could not be traced.
pub type FilesResult = Result<Vec<String>, String>;
/// The requests that a listener has seen.
type Seen = Arc<Mutex<Vec<String>>>;

pub struct Env<'a> {
    /// The repository root: the paths of the inputs are relative to it.
    pub root: &'a Path,
    /// HEAD of the store (information for the case file).
    pub head: &'a dyn Fn() -> Option<String>,
    /// Run the `fa` arguments.
    pub run: &'a dyn Fn(&[String]) -> RunResult,
    /// The files (relative to the root) that the arguments open for reading.
    pub record: &'a dyn Fn(&[String]) -> FilesResult,
    pub today: String,
}

#[derive(Debug, Default)]
pub struct Options {
    pub case: Option<String>,
    pub cases: Option<PathBuf>,
    pub update: Option<String>,
    pub all: bool,
    pub record_inputs: bool,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut o = Options::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "--case" => o.case = Some(value("--case")?),
            "--cases" => o.cases = Some(PathBuf::from(value("--cases")?)),
            "--update" => {
                let r = value("--update")?;
                if r.trim().is_empty() || r.starts_with("--") {
                    return Err("--update needs a reason (the text that says why the golden changes)".into());
                }
                o.update = Some(r);
            }
            "--all" => o.all = true,
            "--record-inputs" => o.record_inputs = true,
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if (o.update.is_some() || o.record_inputs) && o.case.is_none() && !o.all {
        return Err("--update and --record-inputs need --case NAME, or --all for each case".into());
    }
    if o.all && o.case.is_some() {
        return Err("--all and --case do not go together".into());
    }
    if o.update.is_some() && o.record_inputs {
        return Err("--update and --record-inputs do not go together".into());
    }
    Ok(o)
}

/// Run (or update, or record) the cases of `cases_path`. Output goes to `out`.
pub fn run_cases(cases_path: &Path, opts: &Options, env: &Env, out: &mut dyn Write) -> Result<u8, String> {
    let text = std::fs::read_to_string(cases_path).map_err(|e| format!("{}: {e}", cases_path.display()))?;
    let mut file: CaseFile = toml::from_str(&text).map_err(|e| format!("{}: {e}", cases_path.display()))?;
    let dir = cases_path.parent().map(Path::to_path_buf).unwrap_or_default();
    if let Some(n) = &opts.case {
        if !file.cases.iter().any(|c| &c.name == n) {
            return Err(format!("no case named '{n}' in {}", cases_path.display()));
        }
    }
    let (mut passed, mut failed, mut skipped, mut changed, mut selected) = (0, 0, Vec::new(), 0, 0);
    for case in file.cases.iter_mut() {
        if opts.case.as_ref().is_some_and(|n| n != &case.name) {
            continue;
        }
        selected += 1;
        check_mask(case)?;
        check_allowed(&case.args).map_err(|e| format!("case {}: {e}", case.name))?;
        if opts.record_inputs {
            let paths = (env.record)(&case.args).map_err(|e| format!("case {}: {e}", case.name))?;
            case.inputs = hash_inputs(env.root, &paths)?;
            let _ = writeln!(out, "RECORDED {}: {} input files", case.name, case.inputs.len());
            changed += 1;
            continue;
        }
        if opts.update.is_none() {
            if let Some(why) = input_problem(env.root, &case.inputs) {
                let _ = writeln!(out, "SKIPPED {}: {why}", case.name);
                skipped.push(case.name.clone());
                continue;
            }
        }
        let o = (env.run)(&case.args).map_err(|e| format!("case {}: {e}", case.name))?;
        if !o.requests.is_empty() {
            let _ = writeln!(out, "FAIL {}: the command asked for the network (the golden run never downloads): {}", case.name, o.requests.join("; "));
            failed += 1;
            continue;
        }
        if o.code != 0 {
            let tail: Vec<&str> = o.stderr.lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect();
            let _ = writeln!(out, "FAIL {}: the command exited with {}{}", case.name, o.code, if tail.is_empty() { String::new() } else { format!(": {}", tail.join(" | ")) });
            failed += 1;
            continue;
        }
        let masked = apply_mask(&o.stdout, &case.mask);
        let missing: Vec<&String> = case.must_contain.iter().filter(|m| !masked.contains(m.as_str())).collect();
        if !missing.is_empty() {
            let _ = writeln!(out, "FAIL {}: the output lacks {missing:?}", case.name);
            failed += 1;
            continue;
        }
        let hash = sha256_hex(masked.as_bytes());
        let txt = dir.join(format!("{}.txt", case.name));
        if let Some(reason) = &opts.update {
            // The input hashes and the expected text change together.
            case.inputs = case.inputs.iter().map(|i| hash_inputs(env.root, std::slice::from_ref(&i.path)).map(|mut v| v.remove(0))).collect::<Result<_, _>>()?;
            std::fs::write(&txt, &masked).map_err(|e| format!("{}: {e}", txt.display()))?;
            if let Some(h) = (env.head)() {
                case.store = h;
            }
            case.sha256 = hash.clone();
            case.reason = Some(reason.clone());
            case.updated = Some(env.today.clone());
            let _ = writeln!(out, "UPDATED {}: {hash} ({}; {} input hashes)", case.name, txt.display(), case.inputs.len());
            changed += 1;
            continue;
        }
        let expected = std::fs::read_to_string(&txt).map_err(|e| format!("{}: {e} (run --update to write it)", txt.display()))?;
        if sha256_hex(expected.as_bytes()) != case.sha256 {
            let _ = writeln!(out, "FAIL {}: the stored text {} does not match the stored hash (edited by hand?)", case.name, txt.display());
            failed += 1;
        } else if hash == case.sha256 {
            let _ = writeln!(out, "PASS {}: {hash}", case.name);
            passed += 1;
        } else {
            let _ = writeln!(out, "FAIL {}: stored {} but got {hash}", case.name, case.sha256);
            let diffs = first_diffs(&expected, &masked, 20);
            for (n, e, a) in &diffs {
                let _ = writeln!(out, "  line {n}\n    expected: {e}\n    actual:   {a}");
            }
            if diffs.len() == 20 {
                let _ = writeln!(out, "  (only the first 20 differing lines)");
            }
            let _ = writeln!(out, "  If the cause is a file that the input list misses: add it (fa selftest golden --record-inputs --case {}).", case.name);
            failed += 1;
        }
    }
    if opts.update.is_some() || opts.record_inputs {
        let s = toml::to_string(&file).map_err(|e| e.to_string())?;
        std::fs::write(cases_path, s).map_err(|e| format!("{}: {e}", cases_path.display()))?;
        let _ = writeln!(out, "{changed} case(s) written to {}", cases_path.display());
        return Ok(if failed > 0 { 1 } else { 0 });
    }
    let names = if skipped.is_empty() { String::new() } else { format!(" ({})", skipped.join(", ")) };
    let _ = writeln!(out, "golden: {passed} passed, {failed} failed, {} skipped{names}", skipped.len());
    if selected > 0 && passed + failed == 0 {
        let _ = writeln!(out, "WARNING: 0 of {selected} cases ran (all skipped): NOTHING WAS CHECKED");
    }
    Ok(if failed > 0 { 1 } else { 0 })
}

fn git(cache: &Path, args: &[&str]) -> Option<std::process::Output> {
    Command::new("git").arg("-C").arg(cache).args(args).output().ok()
}

/// A local listener that writes down the first line of each connection (a request through the proxy variable).
fn request_listener() -> Result<(u16, Seen), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            let _ = BufReader::new(&stream).read_line(&mut line);
            if let Ok(mut v) = sink.lock() {
                v.push(line.trim().to_string());
            }
            // closing the connection makes the request fail
        }
    });
    Ok((port, seen))
}

/// Entry point: the arguments after `selftest golden`.
pub fn command(repo: &Path, args: &[String]) -> u8 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return 0;
    }
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(m) => {
            eprintln!("fa: {m}\n\n{USAGE}");
            return 64;
        }
    };
    let cache = repo.join("data").join("raw");
    let cases_path = opts.cases.clone().unwrap_or_else(|| repo.join("rust").join("tests").join("golden").join("cases.toml"));
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("fa: {e}");
            return 2;
        }
    };
    let (port, seen) = match request_listener() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("fa: {e}");
            return 2;
        }
    };
    let proxy = format!("http://127.0.0.1:{port}");
    // No download: the proxy is a local listener that records and closes; the repository is the working folder.
    let child = |prefix: &[&str], a: &[String]| -> Command {
        let mut c = match prefix.split_first() {
            Some((p, rest)) => {
                let mut c = Command::new(p);
                c.args(rest).arg(&exe);
                c
            }
            None => Command::new(&exe),
        };
        c.args(a).current_dir(repo).env("FA_REPO", repo).env("HTTPS_PROXY", &proxy).env("https_proxy", &proxy);
        c
    };
    let head = || git(&cache, &["rev-parse", "HEAD"]).filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let run = |a: &[String]| -> RunResult {
        check_allowed(a)?;
        if let Ok(mut v) = seen.lock() {
            v.clear();
        }
        let o = child(&[], a).output().map_err(|e| e.to_string())?;
        let requests = seen.lock().map(|v| v.clone()).unwrap_or_default();
        Ok(RunOutput { stdout: String::from_utf8_lossy(&o.stdout).into_owned(), stderr: String::from_utf8_lossy(&o.stderr).into_owned(), code: o.status.code().unwrap_or(-1), requests })
    };
    let record = |a: &[String]| -> Result<Vec<String>, String> {
        check_allowed(a)?;
        let trace = std::env::temp_dir().join(format!("fa-golden-strace-{}.txt", std::process::id()));
        let st = child(&["strace", "-f", "-e", "trace=openat", "-o", &trace.to_string_lossy()], a)
            .output()
            .map_err(|e| format!("strace is needed for --record-inputs ({e}); write the list by hand and add a `note` to the case"))?;
        let text = std::fs::read_to_string(&trace).map_err(|e| format!("{}: {e}", trace.display()))?;
        let _ = std::fs::remove_file(&trace);
        if !st.status.success() {
            return Err(format!("the command exited with {:?} under strace", st.status.code()));
        }
        // The files that fa itself reads at start (the case file, the binary) are outside the repository data; only paths under the repository count.
        Ok(paths_from_strace(&text, repo))
    };
    let env = Env { root: repo, head: &head, run: &run, record: &record, today: mapcheck::dates::format(mapcheck::dates::today()) };
    match run_cases(&cases_path, &opts, &env, &mut std::io::stdout()) {
        Ok(code) => code,
        Err(m) => {
            eprintln!("fa: {m}");
            // A refused command or a bad case file is a usage fault of the case file.
            if m.contains("allow list") || m.contains("can download") || m.contains("mask") {
                64
            } else {
                2
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        // 56 bytes: the padding needs a second block
        assert_eq!(sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
        // a stream over many buffers: one million 'a'
        let million = vec![b'a'; 1_000_000];
        assert_eq!(sha256_hex(&million), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }

    #[test]
    fn mask_replaces_the_timing_line_only() {
        let mask = [MaskRule { prefix: Some("time:".into()), timing: true, ..Default::default() }, MaskRule { contains: Some("pid".into()), why: Some("varies".into()), ..Default::default() }];
        let got = apply_mask("a 1\ntime: 0.42 s\nb pid 77\nc\n", &mask);
        assert_eq!(got, "a 1\n<masked>\n<masked>\nc\n");
    }

    #[test]
    fn mask_without_why_is_refused() {
        let mut c = Case { name: "x".into(), ..Default::default() };
        c.mask = vec![MaskRule { contains: Some("n=".into()), ..Default::default() }];
        assert!(check_mask(&c).unwrap_err().contains("why"));
        c.mask[0].why = Some("count varies".into());
        assert!(check_mask(&c).is_ok());
    }

    #[test]
    fn allow_list() {
        assert!(check_allowed(&s(&["dcf", "drivers", "IBM"])).is_ok());
        assert!(check_allowed(&s(&["options", "check", "SPX", "--date", "2026-11-20"])).is_ok());
        assert!(check_allowed(&s(&["sync", "IBM"])).unwrap_err().contains("allow list"));
        assert!(check_allowed(&s(&["prices", "get", "IBM"])).is_err());
        assert!(check_allowed(&s(&["dcf", "drivers", "IBM", "--refresh"])).unwrap_err().contains("download"));
    }

    #[test]
    fn first_diffs_limit_and_numbers() {
        let e: String = (0..30).map(|i| format!("l{i}\n")).collect();
        let a: String = (0..30).map(|i| format!("m{i}\n")).collect();
        let d = first_diffs(&e, &a, 20);
        assert_eq!(d.len(), 20);
        assert_eq!(d[0], (1, "l0".to_string(), "m0".to_string()));
        assert_eq!(first_diffs("a\nb\n", "a\n", 20), vec![(2, "b".to_string(), "<no line>".to_string())]);
    }

    #[test]
    fn update_needs_a_reason_and_a_target() {
        assert!(parse_options(&s(&["--update"])).is_err());
        assert!(parse_options(&s(&["--update", "", "--case", "a"])).is_err());
        assert!(parse_options(&s(&["--update", "why"])).unwrap_err().contains("--all"));
        assert!(parse_options(&s(&["--update", "why", "--case", "a"])).is_ok());
        assert!(parse_options(&s(&["--update", "why", "--all"])).is_ok());
        assert!(parse_options(&s(&["--record-inputs"])).is_err());
        assert!(parse_options(&s(&["--record-inputs", "--case", "a"])).is_ok());
        assert!(parse_options(&s(&["--bogus"])).is_err());
        assert_eq!(command(Path::new("/nonexistent"), &s(&["--update"])), 64);
    }

    #[test]
    fn strace_paths_keep_the_files_under_the_root() {
        let root = std::env::temp_dir().join(format!("fa-golden-strace-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data/raw/IBM")).unwrap();
        std::fs::write(root.join("data/raw/IBM/prices.json"), "x").unwrap();
        std::fs::write(root.join("list.txt"), "x").unwrap();
        let r = root.display();
        let trace = format!(
            "10 openat(AT_FDCWD, \"{r}/data/raw/IBM/prices.json\", O_RDONLY|O_CLOEXEC) = 3\n\
             [pid 11] openat(AT_FDCWD, \"list.txt\", O_RDONLY) = 4\n\
             10 openat(AT_FDCWD, \"{r}/data/raw/IBM/missing.json\", O_RDONLY) = -1 ENOENT (No such file or directory)\n\
             10 openat(AT_FDCWD, \"{r}/data/raw/IBM\", O_RDONLY|O_DIRECTORY) = 5\n\
             10 openat(AT_FDCWD, \"{r}/out.txt\", O_WRONLY|O_CREAT) = 6\n\
             10 openat(AT_FDCWD, \"/etc/ld.so.cache\", O_RDONLY) = 7\n\
             10 openat(AT_FDCWD, \"{r}/data/raw/IBM/prices.json\", O_RDONLY) = 8\n"
        );
        assert_eq!(paths_from_strace(&trace, &root), s(&["data/raw/IBM/prices.json", "list.txt"]));
        let _ = std::fs::remove_dir_all(&root);
    }

    struct Fixture {
        dir: PathBuf,
    }

    impl Fixture {
        /// A temporary repository root with one input file `data/raw/in.txt` and a case file with one case.
        fn new(tag: &str, body: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("fa-golden-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("data/raw")).unwrap();
            std::fs::write(dir.join("data/raw/in.txt"), "input one").unwrap();
            let mask = vec![MaskRule { prefix: Some("time".into()), timing: true, ..Default::default() }];
            let masked = apply_mask(body, &mask);
            std::fs::write(dir.join("one.txt"), &masked).unwrap();
            let c = Case {
                name: "one".into(),
                args: s(&["dcf", "drivers", "IBM"]),
                sha256: sha256_hex(masked.as_bytes()),
                mask,
                inputs: vec![Input { path: "data/raw/in.txt".into(), sha256: sha256_hex(b"input one") }],
                ..Default::default()
            };
            std::fs::write(dir.join("cases.toml"), toml::to_string(&CaseFile { cases: vec![c] }).unwrap()).unwrap();
            Fixture { dir }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn run_out(f: &Fixture, opts: &Options, out: RunOutput, calls: &RefCell<u32>) -> (u8, String) {
        let head = || Some("abc123".to_string());
        let run = |_: &[String]| -> RunResult {
            *calls.borrow_mut() += 1;
            Ok(out.clone())
        };
        let record = |_: &[String]| -> Result<Vec<String>, String> { Ok(s(&["data/raw/in.txt"])) };
        let env = Env { root: &f.dir, head: &head, run: &run, record: &record, today: "2026-11-21".into() };
        let mut buf = Vec::new();
        let code = run_cases(&f.dir.join("cases.toml"), opts, &env, &mut buf).unwrap();
        (code, String::from_utf8(buf).unwrap())
    }

    fn run_with(f: &Fixture, opts: &Options, stdout: &str, calls: &RefCell<u32>) -> (u8, String) {
        run_out(f, opts, RunOutput { stdout: stdout.into(), ..Default::default() }, calls)
    }

    #[test]
    fn pass_and_difference() {
        let body = "value 83.84\ntime 1.2 s\nend\n";
        let f = Fixture::new("a", body);
        let calls = RefCell::new(0);
        let (code, text) = run_with(&f, &Options::default(), "value 83.84\ntime 9.9 s\nend\n", &calls);
        assert_eq!(code, 0, "{text}");
        assert!(text.contains("PASS one") && text.contains("1 passed, 0 failed, 0 skipped"));
        assert!(!text.contains("WARNING"));
        let (code, text) = run_with(&f, &Options::default(), "value 84.00\ntime 9.9 s\nend\n", &calls);
        assert_eq!(code, 1);
        assert!(text.contains("line 1") && text.contains("expected: value 83.84") && text.contains("actual:   value 84.00"), "{text}");
        assert!(text.contains("add it"), "{text}");
    }

    #[test]
    fn a_changed_or_missing_input_skips_and_names_the_file() {
        let f = Fixture::new("b", "a\n");
        let calls = RefCell::new(0);
        std::fs::write(f.dir.join("data/raw/in.txt"), "input two").unwrap();
        let (code, text) = run_with(&f, &Options::default(), "a\n", &calls);
        assert_eq!(code, 0);
        assert!(text.contains("SKIPPED one: input data/raw/in.txt has another hash"), "{text}");
        assert!(text.contains("0 passed, 0 failed, 1 skipped (one)") && text.contains("0 of 1 cases ran") && text.contains("NOTHING WAS CHECKED"), "{text}");
        std::fs::remove_file(f.dir.join("data/raw/in.txt")).unwrap();
        let (_, text) = run_with(&f, &Options::default(), "a\n", &calls);
        assert!(text.contains("SKIPPED one: input data/raw/in.txt is missing"), "{text}");
        assert_eq!(*calls.borrow(), 0, "a skipped case does not run the command");
    }

    #[test]
    fn a_download_fails_the_case_and_names_the_request() {
        let body = "a\n";
        let f = Fixture::new("e", body);
        let calls = RefCell::new(0);
        // The output is right, but the command asked for the network: FAIL, not pass, not skip.
        let out = RunOutput { stdout: body.into(), requests: s(&["CONNECT query1.finance.yahoo.com:443 HTTP/1.1"]), ..Default::default() };
        let (code, text) = run_out(&f, &Options::default(), out, &calls);
        assert_eq!(code, 1, "{text}");
        assert!(text.contains("FAIL one") && text.contains("asked for the network") && text.contains("query1.finance.yahoo.com"), "{text}");
        assert!(!text.contains("PASS") && !text.contains("SKIPPED"), "{text}");
    }

    #[test]
    fn the_listener_records_a_proxy_request() {
        let (port, seen) = request_listener().unwrap();
        let mut c = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        c.write_all(b"CONNECT api.example.org:443 HTTP/1.1\r\n\r\n").unwrap();
        drop(c);
        for _ in 0..100 {
            if !seen.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(seen.lock().unwrap().as_slice(), ["CONNECT api.example.org:443 HTTP/1.1"]);
    }

    #[test]
    fn update_writes_input_hashes_reason_and_date_and_a_hand_edited_text_is_caught() {
        let f = Fixture::new("c", "a\nb\n");
        let calls = RefCell::new(0);
        // the input changes; an update takes the new hash and the new expected text together
        std::fs::write(f.dir.join("data/raw/in.txt"), "input two").unwrap();
        let opts = Options { update: Some("model changed".into()), case: Some("one".into()), ..Default::default() };
        let (code, text) = run_with(&f, &opts, "a\nc\n", &calls);
        assert_eq!(code, 0, "{text}");
        let file: CaseFile = toml::from_str(&std::fs::read_to_string(f.dir.join("cases.toml")).unwrap()).unwrap();
        let c = &file.cases[0];
        assert_eq!(c.reason.as_deref(), Some("model changed"));
        assert_eq!(c.updated.as_deref(), Some("2026-11-21"));
        assert_eq!(c.store, "abc123");
        assert_eq!(c.sha256, sha256_hex(b"a\nc\n"));
        assert_eq!(c.inputs[0].sha256, sha256_hex(b"input two"));
        assert_eq!(std::fs::read_to_string(f.dir.join("one.txt")).unwrap(), "a\nc\n");
        let (code, _) = run_with(&f, &Options::default(), "a\nc\n", &calls);
        assert_eq!(code, 0);
        std::fs::write(f.dir.join("one.txt"), "a\nZ\n").unwrap();
        let (code, text) = run_with(&f, &Options::default(), "a\nc\n", &calls);
        assert_eq!(code, 1);
        assert!(text.contains("does not match the stored hash"), "{text}");
    }

    #[test]
    fn record_inputs_writes_the_list_with_hashes() {
        let f = Fixture::new("r", "a\n");
        let mut file: CaseFile = toml::from_str(&std::fs::read_to_string(f.dir.join("cases.toml")).unwrap()).unwrap();
        file.cases[0].inputs.clear();
        std::fs::write(f.dir.join("cases.toml"), toml::to_string(&file).unwrap()).unwrap();
        let calls = RefCell::new(0);
        let opts = Options { record_inputs: true, all: true, ..Default::default() };
        let (code, text) = run_with(&f, &opts, "a\n", &calls);
        assert_eq!(code, 0, "{text}");
        let file: CaseFile = toml::from_str(&std::fs::read_to_string(f.dir.join("cases.toml")).unwrap()).unwrap();
        assert_eq!(file.cases[0].inputs, vec![Input { path: "data/raw/in.txt".into(), sha256: sha256_hex(b"input one") }]);
    }

    #[test]
    fn a_case_outside_the_allow_list_is_refused_before_it_runs() {
        let f = Fixture::new("d", "a\n");
        let mut file: CaseFile = toml::from_str(&std::fs::read_to_string(f.dir.join("cases.toml")).unwrap()).unwrap();
        file.cases[0].args = s(&["sync", "IBM"]);
        std::fs::write(f.dir.join("cases.toml"), toml::to_string(&file).unwrap()).unwrap();
        let calls = RefCell::new(0);
        let head = || Some("abc123".to_string());
        let run = |_: &[String]| -> RunResult {
            *calls.borrow_mut() += 1;
            Ok(RunOutput::default())
        };
        let record = |_: &[String]| -> Result<Vec<String>, String> { Ok(vec![]) };
        let env = Env { root: &f.dir, head: &head, run: &run, record: &record, today: "2026-11-21".into() };
        let err = run_cases(&f.dir.join("cases.toml"), &Options::default(), &env, &mut Vec::new()).unwrap_err();
        assert!(err.contains("allow list"), "{err}");
        assert_eq!(*calls.borrow(), 0);
    }
}
