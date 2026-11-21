//! The run harness of `fa portfolio backtest`: the hypothesis file, the add-only run log with the variant count, the
//! out-of-sample flag, the cost levels, the trade list and the poison check. It adds no engine: the runs use
//! `portfolio::run` and the strategies as they are.

use super::{Config, Perf, Report, Strategy, Trade, View};
use mapcheck::facts::CompanyFacts;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The label of the ticker list, in each output of a stock run until point-in-time index members exist (part 3).
pub const UNIVERSE_LABEL: &str = "universe: names of today (survivor bias)";
/// The label of the price type of a stock run.
pub const PRICE_LABEL: &str = "prices: market prices (closes), not a model price";

// ---- the hypothesis file ----

/// A registered hypothesis (`trading/hypotheses/<id>.toml`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hypothesis {
    pub id: String,
    /// The registration date.
    pub date: String,
    pub hypothesis: String,
    pub strategy: String,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
    /// The tickers file (a path from the repository root, or an absolute path).
    pub universe: String,
    /// The first day of the out-of-sample window.
    pub oos_start: String,
    /// The id of the hypothesis that this one changes (if any).
    #[serde(default)]
    pub parent: Option<String>,
}

/// The parameters that a hypothesis file can set, and the option of the command for each.
const PARAMS: [(&str, &str); 5] = [("paths", "--paths"), ("seed", "--seed"), ("table", "--table"), ("rebalance", "--rebalance"), ("cost_bp", "--cost-bp")];

impl Hypothesis {
    pub fn parse(text: &str) -> Result<Hypothesis, String> {
        let h: Hypothesis = toml::from_str(text).map_err(|e| format!("hypothesis file: {e}"))?;
        if h.id.is_empty() || !h.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') || h.id.starts_with('.') {
            return Err(format!("hypothesis id '{}': letters, digits, '-', '_' and '.' only", h.id));
        }
        for (name, v) in [("date", &h.date), ("oos_start", &h.oos_start)] {
            if mapcheck::dates::parse(v).is_none() {
                return Err(format!("hypothesis {name} '{v}' is not YYYY-MM-DD"));
            }
        }
        if h.hypothesis.trim().is_empty() || h.universe.trim().is_empty() {
            return Err("hypothesis file: the text of the hypothesis and the universe are needed".into());
        }
        if !["value-rank", "momentum"].contains(&h.strategy.as_str()) {
            return Err(format!("hypothesis strategy '{}': value-rank or momentum", h.strategy));
        }
        if let Some(k) = h.params.keys().find(|k| !PARAMS.iter().any(|p| p.0 == k.as_str())) {
            return Err(format!("hypothesis params: unknown key '{k}' (known: paths, seed, table, rebalance, cost_bp)"));
        }
        Ok(h)
    }

    pub fn oos_day(&self) -> i64 {
        mapcheck::dates::parse(&self.oos_start).unwrap_or(i64::MAX)
    }

    /// The tickers file as a path.
    pub fn universe_path(&self, repo: &Path) -> PathBuf {
        let p = PathBuf::from(&self.universe);
        if p.is_absolute() {
            p
        } else {
            repo.join(p)
        }
    }

    /// The arguments of the run: the arguments of the command plus the values of the file. A value of the command that
    /// differs from the file is an error.
    pub fn merge_args(&self, args: &[String], repo: &Path) -> Result<Vec<String>, String> {
        let get = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
        let mut out = args.to_vec();
        let same = |a: &str, b: &str| a == b || matches!((a.parse::<f64>(), b.parse::<f64>()), (Ok(x), Ok(y)) if x == y);
        if let Some(s) = get("--strategy") {
            if s != self.strategy {
                return Err(format!("--strategy {s} conflicts with the hypothesis file ({})", self.strategy));
            }
        } else {
            out.extend(["--strategy".to_string(), self.strategy.clone()]);
        }
        let universe = self.universe_path(repo).to_string_lossy().into_owned();
        match get("--tickers-file") {
            Some(t) if t != universe && t != self.universe => return Err(format!("--tickers-file {t} conflicts with the hypothesis file ({})", self.universe)),
            Some(_) => {}
            None => out.extend(["--tickers-file".to_string(), universe]),
        }
        for (key, opt) in PARAMS {
            let Some(v) = self.params.get(key) else { continue };
            let text = match v {
                toml::Value::String(s) => s.clone(),
                toml::Value::Integer(i) => i.to_string(),
                toml::Value::Float(f) => f.to_string(),
                _ => return Err(format!("hypothesis params.{key}: a number or a text")),
            };
            match get(opt) {
                Some(c) if !same(&c, &text) => return Err(format!("{opt} {c} conflicts with the hypothesis file (params.{key} = {text})")),
                Some(_) => {}
                None => out.extend([opt.to_string(), text]),
            }
        }
        Ok(out)
    }
}

/// What the git history says about a hypothesis file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileProof {
    /// The path from the repository root.
    pub path: String,
    pub commit: String,
    pub commit_time: String,
    pub sha256: String,
}

/// The output of a git command (None if git fails or is not there).
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let o = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// The hypothesis file must be in HEAD and have no change: the file existed before the run (proof of order).
pub fn prove_committed(repo: &Path, file: &Path) -> Result<FileProof, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let canon = file.canonicalize().map_err(|e| format!("{}: {e}", file.display()))?;
    let root = repo.canonicalize().map_err(|e| format!("{}: {e}", repo.display()))?;
    let rel = canon.strip_prefix(&root).map_err(|_| format!("{} is not inside the repository {}", file.display(), root.display()))?;
    let rel = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
    if git(&root, &["cat-file", "-e", &format!("HEAD:{rel}")]).is_none() {
        return Err(format!("{rel} is not committed (in HEAD): commit the hypothesis file before the first run"));
    }
    if !git(&root, &["status", "--porcelain", "--", &rel]).ok_or("git status failed")?.is_empty() {
        return Err(format!("{rel} has a change that is not committed: a changed hypothesis is a new id"));
    }
    let log = git(&root, &["log", "-1", "--format=%H %cI", "--", &rel]).ok_or("git log failed")?;
    let mut w = log.split_whitespace();
    let (commit, commit_time) = (w.next().unwrap_or_default().to_string(), w.next().unwrap_or_default().to_string());
    if commit.is_empty() {
        return Err(format!("{rel}: no commit found"));
    }
    Ok(FileProof { path: rel, commit, commit_time, sha256: sha256_hex(&bytes) })
}

/// The commit of the code and whether the code has a change that is not committed (the run logs are not counted).
pub fn code_state(repo: &Path) -> (Option<String>, bool) {
    let commit = git(repo, &["rev-parse", "HEAD"]);
    let dirty = git(repo, &["status", "--porcelain", "--", ".", ":(exclude)trading/runs"]).is_none_or(|s| !s.is_empty());
    (commit, dirty)
}

/// The commit of the data store, if the cache is the top folder of a git repository (else None), and its dirty flag.
pub fn store_state(cache: &Path) -> (Option<String>, Option<bool>) {
    let (Some(top), Ok(canon)) = (git(cache, &["rev-parse", "--show-toplevel"]), cache.canonicalize()) else { return (None, None) };
    if PathBuf::from(top).canonicalize().ok().as_deref() != Some(canon.as_path()) {
        return (None, None);
    }
    (git(cache, &["rev-parse", "HEAD"]), git(cache, &["status", "--porcelain"]).map(|s| !s.is_empty()))
}

// ---- SHA-256 ----

pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks(4).enumerate() {
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
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

// ---- time, out-of-sample, costs ----

/// `YYYYMMDDTHHMMSSZ` (UTC) of a Unix time in seconds.
pub fn utc_stamp(secs: i64) -> String {
    let (day, r) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    format!("{}T{:02}{:02}{:02}Z", mapcheck::dates::format(day).replace('-', ""), r / 3600, r % 3600 / 60, r % 60)
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// A run that ends after the first day of the out-of-sample window has read that window.
pub fn oos_read(end: i64, oos_start: i64) -> bool {
    end > oos_start
}

/// A strategy that gives the target weights of an earlier run of the same strategy (the weights do not depend on the cost).
pub struct Replay {
    name: String,
    targets: BTreeMap<i64, Vec<(String, f64)>>,
}

impl Replay {
    pub fn new(r: &Report) -> Replay {
        Replay { name: r.strategy.clone(), targets: r.targets.iter().cloned().collect() }
    }
}

impl Strategy for Replay {
    fn name(&self) -> &str {
        &self.name
    }

    fn weights(&self, view: &View, _: &[String]) -> Result<Vec<(String, f64)>, String> {
        self.targets.get(&view.date()).cloned().ok_or_else(|| format!("no recorded weights for {}", mapcheck::dates::format(view.date())))
    }
}

/// The same strategy at 0x and at 2x the cost: (0x, 2x). The target weights and the dates are those of the 1x run.
pub fn cost_levels(prices: &super::Prices, universe: &[String], one: &Report, cfg: &Config) -> Result<(Report, Report), String> {
    let replay = Replay::new(one);
    let at = |k: f64| super::run(prices, universe, &replay, &Config { cost_bp: cfg.cost_bp * k, ..cfg.clone() });
    Ok((at(0.0)?, at(2.0)?))
}

pub fn perf_json(p: &Perf) -> Value {
    json!({ "total": p.total, "annual": p.annual, "volatility": p.vol, "max_drawdown": p.max_drawdown })
}

// ---- the trade list ----

/// The trade list as CSV. A comment line with the labels comes first. `before`: only the trades on days before it.
pub fn trades_csv(trades: &[Trade], before: Option<i64>) -> String {
    let mut s = format!("# {UNIVERSE_LABEL}; {PRICE_LABEL}\ndate,ticker,action,weight_before,weight_after,price,signal,filing_accession\n");
    for t in trades.iter().filter(|t| before.is_none_or(|d| t.date < d)) {
        let (sig, accn) = t.signal.as_ref().map_or((String::new(), ""), |(x, a)| (format!("{x:.6}"), a.as_str()));
        let _ = writeln!(s, "{},{},{},{:.6},{:.6},{:.4},{sig},{accn}", mapcheck::dates::format(t.date), t.ticker, if t.buy { "buy" } else { "sell" }, t.before, t.after, t.price);
    }
    s
}

// ---- the poison check ----

fn tick_hash(t: &str) -> u64 {
    t.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// The factor of the closes after the poison day. It differs by ticker: a uniform factor keeps a rank. One ticker in four
/// gets 1000, so one huge return enters the beta and the momentum of a strategy that reads past the date.
pub fn price_factor(ticker: &str) -> f64 {
    let h = tick_hash(ticker);
    if h % 4 == 2 {
        1000.0
    } else {
        1.5 + 0.5 * ((h >> 8) % 3) as f64
    }
}

/// The factor of the facts filed after the poison day. It differs by ticker.
pub fn fact_factor(ticker: &str) -> f64 {
    1.0 + 400.0 * (1 + (tick_hash(ticker) >> 16) % 11) as f64
}

/// A series (ISO date, close) with each close after `day` (YYYY-MM-DD) multiplied by the factor of the ticker.
pub fn poison_series(series: &[(String, f64)], day: &str, ticker: &str) -> Vec<(String, f64)> {
    let f = price_factor(ticker);
    series.iter().map(|(d, c)| (d.clone(), if d.get(..10).unwrap_or("") > day { c * f } else { *c })).collect()
}

/// Each fact filed after `day` (YYYY-MM-DD): its value multiplied by `factor`.
pub fn poison_facts(facts: &mut CompanyFacts, day: &str, factor: f64) {
    for f in facts.facts.values_mut().flat_map(|ns| ns.values_mut()).flat_map(|c| c.units.values_mut()).flatten() {
        if f.filed.as_deref().and_then(|d| d.get(..10)).is_some_and(|d| d > day) {
            f.val = f.val.map(|v| v * factor);
        }
    }
}

/// Compare the run and the poison run for the rebalances before `before`: the target weights, then the trade list as text.
/// Gives the number of rebalances, or the first difference.
pub fn poison_compare(clean: &Report, poisoned: &Report, before: i64) -> Result<usize, String> {
    let early = |r: &Report| r.targets.iter().filter(|(d, _)| *d < before).cloned().collect::<Vec<_>>();
    let (a, b) = (early(clean), early(poisoned));
    if a.is_empty() {
        return Err("no rebalance before the poison day: nothing to compare".into());
    }
    for i in 0..a.len().max(b.len()) {
        match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) if x == y => {}
            (Some((d, x)), Some((_, y))) => {
                let (mx, my): (BTreeMap<_, _>, BTreeMap<_, _>) = (x.iter().cloned().collect(), y.iter().cloned().collect());
                let t = mx.keys().chain(my.keys()).find(|k| mx.get(*k) != my.get(*k)).cloned().unwrap_or_default();
                return Err(format!("rebalance {}: target weight of {t} is {:?} in the run and {:?} in the poison run", mapcheck::dates::format(*d), mx.get(&t), my.get(&t)));
            }
            _ => return Err(format!("the number of rebalances before the poison day differs: {} and {}", a.len(), b.len())),
        }
    }
    let (ta, tb) = (trades_csv(&clean.trades, Some(before)), trades_csv(&poisoned.trades, Some(before)));
    if ta != tb {
        let line = ta.lines().zip(tb.lines()).find(|(x, y)| x != y).map_or_else(|| format!("{} and {} lines", ta.lines().count(), tb.lines().count()), |(x, y)| format!("'{x}' against '{y}'"));
        return Err(format!("the trade list before the poison day differs: {line}"));
    }
    Ok(a.len())
}

// ---- the run log ----

/// The number of run files of one id, and of all the ids that name the same strategy, in `runs_dir`.
pub fn count_runs(runs_dir: &Path, id: &str, strategy: &str) -> Result<(usize, usize), String> {
    let (mut by_id, mut by_strategy) = (0, 0);
    let Ok(dirs) = std::fs::read_dir(runs_dir) else { return Ok((0, 0)) };
    for dir in dirs.flatten().filter(|d| d.path().is_dir()) {
        let is_id = dir.file_name().to_string_lossy() == id;
        for f in std::fs::read_dir(dir.path()).map_err(|e| format!("{}: {e}", dir.path().display()))?.flatten() {
            let path = f.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            by_id += usize::from(is_id);
            let v: Value = serde_json::from_slice(&std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?).map_err(|e| format!("{}: not a run log ({e})", path.display()))?;
            by_strategy += usize::from(v.get("strategy").and_then(Value::as_str) == Some(strategy));
        }
    }
    Ok((by_id, by_strategy))
}

/// Write the run log `runs_dir/<id>/<stamp>.json`. `make` gets the variant counts (this run included: runs of the id, runs of
/// the strategy) and gives the log. An existing file is never replaced.
pub fn write_run_log(runs_dir: &Path, id: &str, strategy: &str, stamp: &str, make: impl FnOnce(usize, usize) -> Value) -> Result<(PathBuf, usize, usize), String> {
    use std::io::Write;
    let (n_id, n_strategy) = count_runs(runs_dir, id, strategy)?;
    let dir = runs_dir.join(id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{stamp}.json"));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("{}: {e} (a run log is never replaced; wait one second)", path.display()))?;
    let text = serde_json::to_string_pretty(&make(n_id + 1, n_strategy + 1)).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes()).and_then(|_| file.write_all(b"\n")).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((path, n_id + 1, n_strategy + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::{run, Prices, Rebalance, Series};

    const GOOD: &str = r#"
id = "value-rank-sp500-v1"
date = "2026-11-21"
hypothesis = "The top quintile by model value over price beats the equal-weight line."
strategy = "value-rank"
universe = "rust/mapcheck/universe/sp500.txt"
oos_start = "2023-01-01"
[params]
paths = 300
seed = 7
rebalance = "monthly"
cost_bp = 10
"#;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("fa-harness-{name}-{}-{}", std::process::id(), now_secs()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn the_hypothesis_file_is_parsed_and_checked() {
        let h = Hypothesis::parse(GOOD).unwrap();
        assert_eq!((h.id.as_str(), h.strategy.as_str(), h.oos_start.as_str(), h.parent), ("value-rank-sp500-v1", "value-rank", "2023-01-01", None));
        assert_eq!(h.params["paths"].as_integer(), Some(300));
        assert!(Hypothesis::parse(&GOOD.replace("2023-01-01", "2023-13-45")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("value-rank-sp500-v1", "../x")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("strategy = \"value-rank\"", "strategy = \"magic\"")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("paths = 300", "pathz = 300")).is_err());
        assert!(Hypothesis::parse(&format!("{GOOD}\nextra = 1")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("oos_start = \"2023-01-01\"", "")).is_err());
    }

    #[test]
    fn a_value_of_the_command_that_conflicts_with_the_file_is_an_error() {
        let h = Hypothesis::parse(GOOD).unwrap();
        let repo = Path::new("/repo");
        let e = h.merge_args(&args(&["--strategy", "momentum", "--start", "2018-01-01"]), repo).unwrap_err();
        assert!(e.contains("conflicts") && e.contains("--strategy"), "{e}");
        assert!(h.merge_args(&args(&["--paths", "500"]), repo).unwrap_err().contains("--paths"));
        assert!(h.merge_args(&args(&["--cost-bp", "20"]), repo).is_err());
        assert!(h.merge_args(&args(&["--tickers-file", "other.txt"]), repo).is_err());
        // the same value (10 and 10.0) is no conflict; the file fills the rest
        let m = h.merge_args(&args(&["--start", "2018-01-01", "--cost-bp", "10.0", "--strategy", "value-rank"]), repo).unwrap();
        let get = |n: &str| m.iter().position(|a| a == n).map(|i| m[i + 1].clone());
        assert_eq!(get("--paths").as_deref(), Some("300"));
        assert_eq!(get("--seed").as_deref(), Some("7"));
        assert_eq!(get("--rebalance").as_deref(), Some("monthly"));
        assert_eq!(get("--tickers-file").as_deref(), Some("/repo/rust/mapcheck/universe/sp500.txt"));
    }

    #[test]
    fn sha256_known_answers() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(sha256_hex(&[b'a'; 1000]), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
    }

    #[test]
    fn the_stamp_is_utc_and_sorts_by_time() {
        assert_eq!(utc_stamp(1_700_000_000), "20231114T221320Z");
        assert_eq!(utc_stamp(0), "19700101T000000Z");
    }

    #[test]
    fn the_oos_flag_is_set_when_the_end_is_after_the_start_of_the_window() {
        let d = |s: &str| mapcheck::dates::parse(s).unwrap();
        assert!(!oos_read(d("2022-12-30"), d("2023-01-01")));
        assert!(!oos_read(d("2023-01-01"), d("2023-01-01")));
        assert!(oos_read(d("2023-01-02"), d("2023-01-01")));
    }

    #[test]
    fn a_run_log_is_never_overwritten_and_the_counts_increase() {
        let dir = scratch("runs");
        let log = |id: &str, strategy: &str| json!({ "id": id, "strategy": strategy });
        let (p1, n1, m1) = write_run_log(&dir, "h1", "momentum", "20261121T100000Z", |n, m| json!({ "id": "h1", "strategy": "momentum", "n": n, "m": m })).unwrap();
        assert_eq!((n1, m1), (1, 1));
        assert!(p1.ends_with("h1/20261121T100000Z.json"));
        // the same name: refused, and the first file keeps its text
        let before = std::fs::read_to_string(&p1).unwrap();
        assert!(write_run_log(&dir, "h1", "momentum", "20261121T100000Z", |_, _| json!({ "id": "h1", "strategy": "momentum", "n": 99 })).is_err());
        assert_eq!(std::fs::read_to_string(&p1).unwrap(), before);
        assert!(before.contains("\"n\": 1"));
        // a new time: the count of the id is 2; another id of the same strategy: the family count counts both
        let (_, n2, m2) = write_run_log(&dir, "h1", "momentum", "20261121T100001Z", |_, _| log("h1", "momentum")).unwrap();
        assert_eq!((n2, m2), (2, 2));
        let (_, n3, m3) = write_run_log(&dir, "h2", "momentum", "20261121T100002Z", |_, _| log("h2", "momentum")).unwrap();
        assert_eq!((n3, m3), (1, 3));
        let (_, n4, m4) = write_run_log(&dir, "h3", "value-rank", "20261121T100003Z", |_, _| log("h3", "value-rank")).unwrap();
        assert_eq!((n4, m4), (1, 1));
        assert_eq!(count_runs(&dir, "h1", "momentum").unwrap(), (2, 3));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn git_in(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false"]).args(args).output().unwrap().status.success();
        assert!(ok, "git {args:?}");
    }

    #[test]
    fn a_hypothesis_file_that_is_not_committed_is_refused() {
        let repo = scratch("git");
        git_in(&repo, &["init", "-q"]);
        let f = repo.join("h.toml");
        std::fs::write(&f, GOOD).unwrap();
        assert!(prove_committed(&repo, &f).unwrap_err().contains("not committed"));
        git_in(&repo, &["add", "h.toml"]);
        assert!(prove_committed(&repo, &f).unwrap_err().contains("not committed")); // added, not committed
        git_in(&repo, &["commit", "-q", "-m", "x"]);
        let p = prove_committed(&repo, &f).unwrap();
        assert_eq!((p.path.as_str(), p.sha256.as_str()), ("h.toml", sha256_hex(GOOD.as_bytes()).as_str()));
        assert_eq!(p.commit.len(), 40);
        assert!(p.commit_time.contains('T'));
        std::fs::write(&f, format!("{GOOD}\n# changed")).unwrap();
        assert!(prove_committed(&repo, &f).unwrap_err().contains("change"));
        let _ = std::fs::remove_dir_all(&repo);
    }

    // ---- the cost levels on a fixture ----

    fn series(n: i64, f: impl Fn(i64) -> f64) -> Series {
        let d0 = mapcheck::dates::parse("2020-01-27").unwrap();
        (0..n).map(|i| (d0 + i, f(i))).collect()
    }

    /// Alternates between A and B on each rebalance.
    struct Switch(std::sync::atomic::AtomicUsize);
    impl Strategy for Switch {
        fn name(&self) -> &str {
            "switch"
        }
        fn weights(&self, v: &View, _: &[String]) -> Result<Vec<(String, f64)>, String> {
            let i = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let _ = v;
            Ok(vec![(if i.is_multiple_of(2) { "A" } else { "B" }.to_string(), 1.0)])
        }
    }

    fn fixture() -> (Prices, Vec<String>, Config) {
        let items = vec![("SPY", series(200, |_| 50.0)), ("A", series(200, |i| 100.0 + i as f64)), ("B", series(200, |i| 100.0 + 0.5 * i as f64))];
        let d0 = mapcheck::dates::parse("2020-01-27").unwrap();
        (Prices(items.into_iter().map(|(t, s)| (t.to_string(), s)).collect()), vec!["A".into(), "B".into()], Config { start: d0, end: d0 + 199, rebalance: Rebalance::Monthly, cost_bp: 50.0 })
    }

    #[test]
    fn the_return_falls_as_the_cost_rises() {
        let (p, u, cfg) = fixture();
        let one = run(&p, &u, &Switch(Default::default()), &cfg).unwrap();
        let (zero, two) = cost_levels(&p, &u, &one, &cfg).unwrap();
        assert!(one.trades.len() > 4);
        assert!(zero.fund.annual > one.fund.annual, "{} {}", zero.fund.annual, one.fund.annual);
        assert!(one.fund.annual > two.fund.annual, "{} {}", one.fund.annual, two.fund.annual);
        assert_eq!((zero.cost_total, zero.dates.clone()), (0.0, one.dates.clone()));
        assert_eq!(two.dates, one.dates);
        assert!((two.cost_total - 2.0 * one.cost_total).abs() < 0.2 * one.cost_total); // about twice
        assert_eq!(zero.targets, one.targets);
    }

    #[test]
    fn the_trade_list_has_the_weights_the_price_and_the_labels() {
        let (p, u, cfg) = fixture();
        let one = run(&p, &u, &Switch(Default::default()), &cfg).unwrap();
        let csv = trades_csv(&one.trades, None);
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some("# universe: names of today (survivor bias); prices: market prices (closes), not a model price"));
        assert_eq!(lines.next(), Some("date,ticker,action,weight_before,weight_after,price,signal,filing_accession"));
        assert_eq!(lines.next(), Some("2020-01-27,A,buy,0.000000,0.999975,100.0000,,"));
        // the switch to B: A is sold, B is bought
        assert!(csv.contains(",A,sell,") && csv.contains(",B,buy,"));
        assert!(trades_csv(&one.trades, Some(cfg.start)).lines().count() == 2);
    }

    // ---- the poison check ----

    /// A strategy that reads the closes of the whole series, past the date (a look-ahead): the weight of the ticker with the
    /// highest last close.
    struct Peek<'a>(&'a BTreeMap<String, Vec<(String, f64)>>, bool);
    impl Strategy for Peek<'_> {
        fn name(&self) -> &str {
            "peek"
        }
        fn weights(&self, v: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
            let date = mapcheck::dates::format(v.date());
            let last = |t: &String| {
                let s = &self.0[t];
                // the honest one reads the days before the date; the cheat reads the end of the series
                if self.1 { s.last().unwrap().1 } else { s[..s.partition_point(|(d, _)| *d < date)].last().map_or(0.0, |x| x.1) }
            };
            let best = universe.iter().max_by(|a, b| last(a).total_cmp(&last(b))).unwrap();
            Ok(vec![(best.clone(), 1.0)])
        }
    }

    #[test]
    fn the_poison_check_passes_for_a_strategy_that_keeps_to_the_date_and_fails_for_one_that_looks_ahead() {
        let raw: BTreeMap<String, Vec<(String, f64)>> = [("SPY", 50.0, 0.0), ("A", 100.0, 1.0), ("B", 100.0, 0.7), ("C", 100.0, 0.4), ("D", 100.0, 0.1)]
            .iter()
            .map(|(t, base, slope)| (t.to_string(), series(200, |i| base + slope * i as f64).iter().map(|(d, c)| (mapcheck::dates::format(*d), *c)).collect()))
            .collect();
        let day = "2020-04-01";
        let before = mapcheck::dates::parse(day).unwrap();
        let poisoned: BTreeMap<String, Vec<(String, f64)>> = raw.iter().map(|(t, s)| (t.clone(), poison_series(s, day, t))).collect();
        let u: Vec<String> = ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect();
        let d0 = mapcheck::dates::parse("2020-01-27").unwrap();
        let cfg = Config { start: d0, end: before - 1, rebalance: Rebalance::Monthly, cost_bp: 10.0 };
        for (cheat, ok) in [(false, true), (true, false)] {
            let a = run(&Prices::from_raw(&raw), &u, &Peek(&raw, cheat), &cfg).unwrap();
            let b = run(&Prices::from_raw(&poisoned), &u, &Peek(&poisoned, cheat), &cfg).unwrap();
            let r = poison_compare(&a, &b, before);
            assert_eq!(r.is_ok(), ok, "{r:?}");
            if ok {
                assert_eq!(r, Ok(3)); // Jan 27, Feb 1, Mar 1
            } else {
                assert!(r.unwrap_err().contains("target weight"));
            }
        }
        // no rebalance before the day: an error, not a pass
        let a = run(&Prices::from_raw(&raw), &u, &Peek(&raw, false), &cfg).unwrap();
        assert!(poison_compare(&a, &a, d0).is_err());
    }

    #[test]
    fn poison_factors_differ_by_ticker_and_poison_only_what_is_after_the_day() {
        let f: std::collections::BTreeSet<u64> = ["AAPL", "IBM", "JNJ", "MSFT", "XOM", "KO", "PG", "T"].iter().map(|t| fact_factor(t).to_bits()).collect();
        assert!(f.len() >= 5);
        let s = vec![("2020-03-31".to_string(), 10.0), ("2020-04-01".to_string(), 10.0), ("2020-04-02".to_string(), 10.0)];
        let p = poison_series(&s, "2020-04-01", "IBM");
        assert_eq!((p[0].1, p[1].1), (10.0, 10.0));
        assert_eq!(p[2].1, 10.0 * price_factor("IBM"));
        assert!(price_factor("IBM") > 1.0);
        let mut facts: CompanyFacts = serde_json::from_str(r#"{"facts":{"us-gaap":{"Revenues":{"units":{"USD":[
            {"end":"2019-12-31","val":100,"form":"10-K","filed":"2020-02-20"},
            {"end":"2020-12-31","val":120,"form":"10-K","filed":"2021-02-19"},
            {"end":"2020-12-31","val":130,"form":"10-K"}]}}}}}"#).unwrap();
        poison_facts(&mut facts, "2020-12-31", 1000.0);
        let v: Vec<Option<f64>> = facts.facts["us-gaap"]["Revenues"].units["USD"].iter().map(|f| f.val).collect();
        assert_eq!(v, [Some(100.0), Some(120_000.0), Some(130.0)]);
    }
}
