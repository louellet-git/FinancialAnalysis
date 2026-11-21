//! `fa selftest golden`: stored commands with the SHA-256 of their expected stdout.
//!
//! One case file (default `rust/tests/golden/cases.toml`) lists the cases. Each case has the `fa` arguments, the commit
//! of the data store that it needs, a mask (lines that vary by design: timing lines; any other mask needs a `why`) and
//! the SHA-256 of the masked stdout. The masked expected text is stored next to the case file (`<name>.txt`), so a
//! difference can show the first lines that differ. The runner never downloads: it runs only commands of a small
//! allow list that read the cache, it refuses the flags that fetch, and it points the proxy of the child to a closed
//! port (the HTTP client reads `HTTPS_PROXY`), so a hidden request fails.
//!
//! A case whose store commit is not the HEAD of the store is SKIPPED and named. It is not a pass.
//! Exit codes: 0 all pass or skipped, 1 a difference, 2 an error of the run, 64 usage.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const USAGE: &str = "\
fa selftest golden [--case NAME] [--cases PATH] [--update REASON [--case NAME | --all]]
  Run the stored cases with this binary and compare the SHA-256 of the masked stdout with the stored hash.
  Default case file: rust/tests/golden/cases.toml of the repository (FA_REPO). The data cache is data/raw of it.
  A case whose `store` commit is not the HEAD of the store is SKIPPED (named, not passed).
  Exit 0: all pass or skipped (the skipped count is printed). Exit 1: a difference (the first 20 lines that differ).
  --case NAME       run only this case
  --cases PATH      another case file (the expected texts are next to it)
  --update REASON   write the new hash and expected text of the named cases, with the reason and the date.
                    A reason is required. Needs --case NAME or --all. Sets `store` to the HEAD of the store.
  The mask has two forms: `prefix` (a line that starts with it) and `contains` (a line that has it). A masked line is
  replaced by `<masked>`. A mask that is not a timing line (`timing = true`) needs a `why`. A case can list
  `must_contain` texts that must be in the output (for example the headline numbers).
  The runner never downloads: only commands of an allow list that read the cache (dcf drivers|attribute, options check,
  trade backtest, vol fit), and no flag that fetches. Not in CI: the store is private.";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Case {
    pub name: String,
    pub args: Vec<String>,
    pub store: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_contain: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mask: Vec<MaskRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
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

/// The SHA-256 of `data` as 64 lower-case hex digits.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
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
    h.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:08x}");
        s
    })
}

// ---------------------------------------------------------------------------------------------
// Mask, allow list, diff
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

// ---------------------------------------------------------------------------------------------
// Environment of a run (the store and the child are replaceable in tests)
// ---------------------------------------------------------------------------------------------

/// The stdout and the exit code of one child run.
pub type RunResult = Result<(String, i32), String>;

pub struct Env<'a> {
    /// Does the store have this commit?
    pub has_commit: &'a dyn Fn(&str) -> bool,
    /// HEAD of the store.
    pub head: &'a dyn Fn() -> Option<String>,
    /// Run the `fa` arguments: stdout and the exit code.
    pub run: &'a dyn Fn(&[String]) -> RunResult,
    pub today: String,
}

#[derive(Debug, Default)]
pub struct Options {
    pub case: Option<String>,
    pub cases: Option<PathBuf>,
    pub update: Option<String>,
    pub all: bool,
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
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if o.update.is_some() && o.case.is_none() && !o.all {
        return Err("--update needs --case NAME, or --all for each case".into());
    }
    if o.all && o.case.is_some() {
        return Err("--all and --case do not go together".into());
    }
    Ok(o)
}

/// Run (or update) the cases of `cases_path`. Output goes to `out`.
pub fn run_cases(cases_path: &Path, opts: &Options, env: &Env, out: &mut dyn Write) -> Result<u8, String> {
    let text = std::fs::read_to_string(cases_path).map_err(|e| format!("{}: {e}", cases_path.display()))?;
    let mut file: CaseFile = toml::from_str(&text).map_err(|e| format!("{}: {e}", cases_path.display()))?;
    let dir = cases_path.parent().map(Path::to_path_buf).unwrap_or_default();
    if let Some(n) = &opts.case {
        if !file.cases.iter().any(|c| &c.name == n) {
            return Err(format!("no case named '{n}' in {}", cases_path.display()));
        }
    }
    let (mut passed, mut failed, mut skipped, mut updated) = (0, 0, Vec::new(), 0);
    for case in file.cases.iter_mut() {
        if opts.case.as_ref().is_some_and(|n| n != &case.name) {
            continue;
        }
        check_mask(case)?;
        check_allowed(&case.args).map_err(|e| format!("case {}: {e}", case.name))?;
        let head = (env.head)();
        let present = (env.has_commit)(&case.store);
        let updating = opts.update.is_some();
        // An update runs on the store that it records. A run needs the store commit of the case.
        if !updating && !(present && head.as_deref().is_some_and(|h| h.starts_with(&case.store) || case.store.starts_with(h))) {
            let why = if present { "the store HEAD is a different commit" } else { "the store commit is not present" };
            let _ = writeln!(out, "SKIPPED {}: needs store {} ({why})", case.name, short(&case.store));
            skipped.push(case.name.clone());
            continue;
        }
        let (stdout, code) = (env.run)(&case.args).map_err(|e| format!("case {}: {e}", case.name))?;
        if code != 0 {
            let _ = writeln!(out, "FAIL {}: the command exited with {code}", case.name);
            failed += 1;
            continue;
        }
        let masked = apply_mask(&stdout, &case.mask);
        let missing: Vec<&String> = case.must_contain.iter().filter(|m| !masked.contains(m.as_str())).collect();
        if !missing.is_empty() {
            let _ = writeln!(out, "FAIL {}: the output lacks {missing:?}", case.name);
            failed += 1;
            continue;
        }
        let hash = sha256_hex(masked.as_bytes());
        let txt = dir.join(format!("{}.txt", case.name));
        if let Some(reason) = &opts.update {
            let h = head.ok_or("the store has no HEAD (is the cache a git clone?)")?;
            std::fs::write(&txt, &masked).map_err(|e| format!("{}: {e}", txt.display()))?;
            case.store = h;
            case.sha256 = hash.clone();
            case.reason = Some(reason.clone());
            case.updated = Some(env.today.clone());
            let _ = writeln!(out, "UPDATED {}: {hash} ({})", case.name, txt.display());
            updated += 1;
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
            failed += 1;
        }
    }
    if opts.update.is_some() {
        let s = toml::to_string(&file).map_err(|e| e.to_string())?;
        std::fs::write(cases_path, s).map_err(|e| format!("{}: {e}", cases_path.display()))?;
        let _ = writeln!(out, "{updated} case(s) updated; each updated case has its reason and date in {}", cases_path.display());
        return Ok(if failed > 0 { 1 } else { 0 });
    }
    let _ = writeln!(out, "golden: {passed} passed, {failed} failed, {} skipped{}", skipped.len(), if skipped.is_empty() { String::new() } else { format!(" ({})", skipped.join(", ")) });
    Ok(if failed > 0 { 1 } else { 0 })
}

fn short(commit: &str) -> &str {
    commit.get(..10).unwrap_or(commit)
}

fn git(cache: &Path, args: &[&str]) -> Option<std::process::Output> {
    Command::new("git").arg("-C").arg(cache).args(args).output().ok()
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
    let has_commit = |c: &str| !c.starts_with('-') && git(&cache, &["cat-file", "-e", &format!("{c}^{{commit}}")]).is_some_and(|o| o.status.success());
    let head = || git(&cache, &["rev-parse", "HEAD"]).filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let run = |a: &[String]| -> Result<(String, i32), String> {
        check_allowed(a)?;
        // No download: a closed port as proxy, and the repository as the working folder (relative paths in the arguments).
        let o = Command::new(&exe)
            .args(a)
            .current_dir(repo)
            .env("FA_REPO", repo)
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .env("https_proxy", "http://127.0.0.1:9")
            .output()
            .map_err(|e| e.to_string())?;
        Ok((String::from_utf8_lossy(&o.stdout).into_owned(), o.status.code().unwrap_or(-1)))
    };
    let env = Env { has_commit: &has_commit, head: &head, run: &run, today: mapcheck::dates::format(mapcheck::dates::today()) };
    match run_cases(&cases_path, &opts, &env, &mut std::io::stdout()) {
        Ok(code) => code,
        Err(m) => {
            eprintln!("fa: {m}");
            // A refused command or a bad case file is a usage fault of the case file.
            if m.contains("allow list") || m.contains("can download") || m.contains("mask") { 64 } else { 2 }
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
        assert!(parse_options(&s(&["--bogus"])).is_err());
        assert_eq!(command(Path::new("/nonexistent"), &s(&["--update"])), 64);
    }

    struct Fixture {
        dir: PathBuf,
    }

    impl Fixture {
        fn new(tag: &str, store: &str, body: &str) -> Fixture {
            let dir = std::env::temp_dir().join(format!("fa-golden-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let masked = apply_mask(body, &[MaskRule { prefix: Some("time".into()), timing: true, ..Default::default() }]);
            std::fs::write(dir.join("one.txt"), &masked).unwrap();
            let c = Case {
                name: "one".into(),
                args: s(&["dcf", "drivers", "IBM"]),
                store: store.into(),
                sha256: sha256_hex(masked.as_bytes()),
                mask: vec![MaskRule { prefix: Some("time".into()), timing: true, ..Default::default() }],
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

    fn run_with(f: &Fixture, opts: &Options, present: bool, stdout: &str, calls: &RefCell<u32>) -> (u8, String) {
        let has = move |_: &str| present;
        let head = || Some("abc123".to_string());
        let out_text = stdout.to_string();
        let run = |_: &[String]| -> Result<(String, i32), String> {
            *calls.borrow_mut() += 1;
            Ok((out_text.clone(), 0))
        };
        let env = Env { has_commit: &has, head: &head, run: &run, today: "2026-11-21".into() };
        let mut buf = Vec::new();
        let code = run_cases(&f.dir.join("cases.toml"), opts, &env, &mut buf).unwrap();
        (code, String::from_utf8(buf).unwrap())
    }

    #[test]
    fn pass_skip_and_difference() {
        let body = "value 83.84\ntime 1.2 s\nend\n";
        let f = Fixture::new("a", "abc123", body);
        let calls = RefCell::new(0);
        let (code, text) = run_with(&f, &Options::default(), true, "value 83.84\ntime 9.9 s\nend\n", &calls);
        assert_eq!(code, 0, "{text}");
        assert!(text.contains("PASS one") && text.contains("1 passed, 0 failed, 0 skipped"));
        // a difference: exit 1 and the differing line
        let (code, text) = run_with(&f, &Options::default(), true, "value 84.00\ntime 9.9 s\nend\n", &calls);
        assert_eq!(code, 1);
        assert!(text.contains("line 1") && text.contains("expected: value 83.84") && text.contains("actual:   value 84.00"), "{text}");
        // a missing store commit: skipped, named, the command did not run
        let before = *calls.borrow();
        let (code, text) = run_with(&f, &Options::default(), false, "x", &calls);
        assert_eq!(code, 0);
        assert!(text.contains("SKIPPED one") && text.contains("1 skipped (one)") && text.contains("0 passed"), "{text}");
        assert_eq!(*calls.borrow(), before);
        // a different HEAD: skipped too
        let g = Fixture::new("b", "fffff0", body);
        let (_, text) = run_with(&g, &Options::default(), true, body, &calls);
        assert!(text.contains("SKIPPED one") && text.contains("different commit"), "{text}");
    }

    #[test]
    fn hand_edited_text_is_caught_and_update_writes_reason() {
        let body = "a\nb\n";
        let f = Fixture::new("c", "abc123", body);
        let calls = RefCell::new(0);
        let opts = Options { update: Some("model changed".into()), case: Some("one".into()), ..Default::default() };
        let (code, text) = run_with(&f, &opts, true, "a\nc\n", &calls);
        assert_eq!(code, 0, "{text}");
        let file: CaseFile = toml::from_str(&std::fs::read_to_string(f.dir.join("cases.toml")).unwrap()).unwrap();
        let c = &file.cases[0];
        assert_eq!(c.reason.as_deref(), Some("model changed"));
        assert_eq!(c.updated.as_deref(), Some("2026-11-21"));
        assert_eq!(c.sha256, sha256_hex(b"a\nc\n"));
        assert_eq!(std::fs::read_to_string(f.dir.join("one.txt")).unwrap(), "a\nc\n");
        // the new golden passes
        let (code, _) = run_with(&f, &Options::default(), true, "a\nc\n", &calls);
        assert_eq!(code, 0);
        // the text edited by hand no longer matches the hash
        std::fs::write(f.dir.join("one.txt"), "a\nZ\n").unwrap();
        let (code, text) = run_with(&f, &Options::default(), true, "a\nc\n", &calls);
        assert_eq!(code, 1);
        assert!(text.contains("does not match the stored hash"), "{text}");
    }

    #[test]
    fn a_case_outside_the_allow_list_is_refused_before_it_runs() {
        let f = Fixture::new("d", "abc123", "a\n");
        let mut file: CaseFile = toml::from_str(&std::fs::read_to_string(f.dir.join("cases.toml")).unwrap()).unwrap();
        file.cases[0].args = s(&["sync", "IBM"]);
        std::fs::write(f.dir.join("cases.toml"), toml::to_string(&file).unwrap()).unwrap();
        let calls = RefCell::new(0);
        let has = |_: &str| true;
        let head = || Some("abc123".to_string());
        let run = |_: &[String]| -> Result<(String, i32), String> {
            *calls.borrow_mut() += 1;
            Ok((String::new(), 0))
        };
        let env = Env { has_commit: &has, head: &head, run: &run, today: "2026-11-21".into() };
        let err = run_cases(&f.dir.join("cases.toml"), &Options::default(), &env, &mut Vec::new()).unwrap_err();
        assert!(err.contains("allow list"), "{err}");
        assert_eq!(*calls.borrow(), 0);
    }
}
