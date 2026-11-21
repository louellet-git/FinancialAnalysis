//! The run harness of `fa portfolio backtest`: the hypothesis file, the add-only run log with the variant counts, the
//! out-of-sample lock, the cost levels, the trade list and the poison check. It adds no engine: the runs use
//! `portfolio::run` and the strategies as they are.

use super::{Config, Perf, Report, Strategy, Trade, View};
use mapcheck::facts::CompanyFacts;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The label of the ticker list, in each output of a stock run until point-in-time index members exist (part 3).
pub const UNIVERSE_LABEL: &str = "universe: names of today (survivor bias)";
/// The label of the price type of a stock run.
pub const PRICE_LABEL: &str = "prices: market prices (closes), not a model price";
/// What the poison check does not cover (the inputs that it does not change).
pub const POISON_NOT_COVERED: &str = "not covered by the poison check: the GSW curve, the parameter table and the tickers file (they are not changed)";
/// The commit and the state of the code at the time of the build (build.rs).
pub const BUILD_COMMIT: &str = match option_env!("FA_BUILD_COMMIT") {
    Some(c) => c,
    None => "",
};
pub const BUILD_DIRTY: bool = match option_env!("FA_BUILD_DIRTY") {
    Some(s) => s.len() == 4, // "true"
    None => false,
};

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
    /// The first day of the out-of-sample window (the window of the oldest ancestor is used: see `resolve`).
    pub oos_start: String,
    /// The number of variants tried before this file (exploratory runs, other starts, other parameters). Needed in the
    /// file that runs, even if 0; an older ancestor file that has no value counts as 0.
    #[serde(default)]
    pub prior_variants: Option<u32>,
    /// The id of the hypothesis that this one changes (if any).
    #[serde(default)]
    pub parent: Option<String>,
    /// The family of the variants (default: the family of the parent, else the name of the strategy).
    #[serde(default)]
    pub family: Option<String>,
    /// True if the person or an agent saw results of the out-of-sample window before this file.
    #[serde(default)]
    pub oos_prior_exposure: bool,
    /// The id of the hypothesis whose window this one uses (it must be the parent): the window was read before.
    #[serde(default)]
    pub oos_inherits: Option<String>,
    /// The first and the last day of the test (YYYY-MM-DD). If present, the command takes them from the file.
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    /// The success criterion, written before the run.
    #[serde(default)]
    pub success: Option<String>,
}

/// The parameters that a hypothesis file can set, and the option of the command for each.
const PARAMS: [(&str, &str); 5] = [("paths", "--paths"), ("seed", "--seed"), ("table", "--table"), ("rebalance", "--rebalance"), ("cost_bp", "--cost-bp")];

impl Hypothesis {
    /// Parse the file that runs: `prior_variants` is needed.
    pub fn parse(text: &str) -> Result<Hypothesis, String> {
        let h = Hypothesis::parse_lenient(text)?;
        if h.prior_variants.is_none() {
            return Err("hypothesis file: prior_variants is needed (the number of variants tried before this file, even if 0)".into());
        }
        Ok(h)
    }

    /// Parse an ancestor or another file of the folder: a file that was registered before `prior_variants` existed
    /// stays readable (its value counts as 0).
    pub fn parse_lenient(text: &str) -> Result<Hypothesis, String> {
        let h: Hypothesis = toml::from_str(text).map_err(|e| format!("hypothesis file: {e}"))?;
        if h.id.is_empty() || !h.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') || h.id.starts_with('.') {
            return Err(format!("hypothesis id '{}': letters, digits, '-', '_' and '.' only", h.id));
        }
        let dates = [("date", Some(&h.date)), ("oos_start", Some(&h.oos_start)), ("start", h.start.as_ref()), ("end", h.end.as_ref())];
        for (name, v) in dates.iter().filter_map(|(n, v)| v.map(|v| (n, v))) {
            if mapcheck::dates::parse(v).is_none() {
                return Err(format!("hypothesis {name} '{v}' is not YYYY-MM-DD"));
            }
        }
        if h.hypothesis.trim().is_empty() || h.universe.trim().is_empty() {
            return Err("hypothesis file: the text of the hypothesis and the universe are needed".into());
        }
        if !["value-rank", "value-rank-weekly", "momentum"].contains(&h.strategy.as_str()) {
            return Err(format!("hypothesis strategy '{}': value-rank, value-rank-weekly or momentum", h.strategy));
        }
        if let Some(k) = h.params.keys().find(|k| !PARAMS.iter().any(|p| p.0 == k.as_str())) {
            return Err(format!("hypothesis params: unknown key '{k}' (known: paths, seed, table, rebalance, cost_bp)"));
        }
        if h.oos_inherits.is_some() && h.oos_inherits != h.parent {
            return Err("hypothesis: oos_inherits must be the id of the parent".into());
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
        let mut put = |opt: &str, key: &str, value: String| -> Result<(), String> {
            match get(opt) {
                Some(c) if !same(&c, &value) => Err(format!("{opt} {c} conflicts with the hypothesis file ({key} = {value})")),
                Some(_) => Ok(()),
                None => {
                    out.extend([opt.to_string(), value]);
                    Ok(())
                }
            }
        };
        put("--strategy", "strategy", self.strategy.clone())?;
        let universe = self.universe_path(repo).to_string_lossy().into_owned();
        match get("--tickers-file") {
            Some(t) if t != universe && t != self.universe => return Err(format!("--tickers-file {t} conflicts with the hypothesis file (universe = {})", self.universe)),
            Some(_) => {}
            None => put("--tickers-file", "universe", universe)?,
        }
        for (key, value) in [("start", &self.start), ("end", &self.end)] {
            if let Some(v) = value {
                put(if key == "start" { "--start" } else { "--end" }, key, v.clone())?;
            }
        }
        for (key, opt) in PARAMS {
            let Some(v) = self.params.get(key) else { continue };
            let text = match v {
                toml::Value::String(s) => s.clone(),
                toml::Value::Integer(i) => i.to_string(),
                toml::Value::Float(f) => f.to_string(),
                _ => return Err(format!("hypothesis params.{key}: a number or a text")),
            };
            put(opt, &format!("params.{key}"), text)?;
        }
        Ok(out)
    }
}

/// The hypothesis with its ancestors: the family and the out-of-sample window.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub family: String,
    /// The first day of the window: the `oos_start` of the oldest ancestor.
    pub window_start: i64,
    /// The id that owns the window.
    pub window_id: String,
    /// The ids from this hypothesis to the oldest ancestor.
    pub chain: Vec<String>,
}

fn hypotheses_dir(repo: &Path) -> PathBuf {
    repo.join("trading").join("hypotheses")
}

/// Follow `parent` to the oldest ancestor (the files are read from the working tree).
pub fn resolve(repo: &Path, h: &Hypothesis) -> Result<Resolved, String> {
    let mut chain = vec![h.id.clone()];
    let mut family = h.family.clone();
    let (mut window_start, mut window_id) = (h.oos_day(), h.id.clone());
    let mut cur = h.clone();
    while let Some(p) = cur.parent.clone() {
        if chain.contains(&p) || chain.len() > 20 {
            return Err(format!("hypothesis parents form a loop at '{p}'"));
        }
        let file = hypotheses_dir(repo).join(format!("{p}.toml"));
        let text = std::fs::read_to_string(&file).map_err(|e| format!("parent hypothesis '{p}': {}: {e}", file.display()))?;
        let child_family = cur.family.clone();
        cur = Hypothesis::parse_lenient(&text)?;
        if let (Some(a), Some(b)) = (&child_family, &cur.family) {
            if a != b {
                return Err(format!("hypothesis family '{a}' differs from the family '{b}' of its parent '{p}': a child stays in the family of its parent"));
            }
        }
        family = family.or_else(|| cur.family.clone());
        (window_start, window_id) = (cur.oos_day(), cur.id.clone());
        chain.push(p);
    }
    Ok(Resolved { family: family.unwrap_or_else(|| h.strategy.clone()), window_start, window_id, chain })
}

/// Whether the out-of-sample window is clean, and the reasons if it is not.
pub fn oos_clean(h: &Hypothesis, window_id: &str, earlier_family_reads: &[String], exploratory_dates: &[String], table_in_sample: bool) -> (bool, Vec<String>) {
    let mut why = Vec::new();
    if h.oos_prior_exposure {
        why.push("oos_prior_exposure: results of the window were seen before the hypothesis file".to_string());
    }
    if h.oos_inherits.is_some() || window_id != h.id {
        why.push(format!("the window is inherited from '{window_id}', which was read before"));
    }
    if !earlier_family_reads.is_empty() {
        why.push(format!("an earlier run of the family read the window ({} run(s), first: {})", earlier_family_reads.len(), earlier_family_reads[0]));
    }
    for date in exploratory_dates {
        why.push(format!("not clean (exploratory read on {date})"));
    }
    if table_in_sample {
        why.push("the parameter table was fitted on all years of the cache (through 2025): the window is in the fit sample of the model".to_string());
    }
    (why.is_empty(), why)
}

/// What the git history says about a hypothesis file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileProof {
    /// The path from the repository root.
    pub path: String,
    pub commit: String,
    pub commit_time: String,
    pub sha256: String,
    /// The text of the file in HEAD (the run uses this text).
    pub text: String,
}

/// The output of a git command (None if git fails or is not there).
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    git_bytes(dir, args).map(|b| String::from_utf8_lossy(&b).trim().to_string())
}

fn git_bytes(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let o = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    o.status.success().then_some(o.stdout)
}

/// The hypothesis file must be in HEAD, with the same bytes in the working tree: the file existed before the run (proof of order).
pub fn prove_committed(repo: &Path, file: &Path) -> Result<FileProof, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let canon = file.canonicalize().map_err(|e| format!("{}: {e}", file.display()))?;
    let root = repo.canonicalize().map_err(|e| format!("{}: {e}", repo.display()))?;
    let rel = canon.strip_prefix(&root).map_err(|_| format!("{} is not inside the repository {}", file.display(), root.display()))?;
    let rel = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
    let Some(head) = git_bytes(&root, &["show", &format!("HEAD:{rel}")]) else {
        return Err(format!("{rel} is not committed (in HEAD): commit the hypothesis file before the first run"));
    };
    if head != bytes || !git(&root, &["status", "--porcelain", "--", &rel]).ok_or("git status failed")?.is_empty() {
        return Err(format!("{rel} has a change that is not committed: a changed hypothesis is a new id"));
    }
    let log = git(&root, &["log", "-1", "--format=%H %cI", "--", &rel]).ok_or("git log failed")?;
    let mut w = log.split_whitespace();
    let (commit, commit_time) = (w.next().unwrap_or_default().to_string(), w.next().unwrap_or_default().to_string());
    if commit.is_empty() {
        return Err(format!("{rel}: no commit found"));
    }
    Ok(FileProof { path: rel, commit, commit_time, sha256: sha256_hex(&head), text: String::from_utf8_lossy(&head).into_owned() })
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

/// A path for the log: from the repository root if it is inside the repository, else only a marker (no path of a machine).
pub fn rel_path(repo: &Path, p: &str) -> String {
    let path = Path::new(p);
    if !path.is_absolute() {
        return p.to_string();
    }
    let root = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    match path.canonicalize().unwrap_or_else(|_| path.to_path_buf()).strip_prefix(&root) {
        Ok(r) => r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"),
        Err(_) => "<path outside the repository>".to_string(),
    }
}

/// The arguments for the log: no absolute path. The value of `--cache` is "<store>".
pub fn log_args(repo: &Path, args: &[String]) -> Vec<String> {
    args.iter().enumerate().map(|(i, a)| if i > 0 && args[i - 1] == "--cache" { "<store>".to_string() } else { rel_path(repo, a) }).collect()
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

/// A run that ends on or after the first day of the out-of-sample window has read that window (the last day of the run
/// is a day of the series).
pub fn oos_read(end: i64, oos_start: i64) -> bool {
    end >= oos_start
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

/// A note on the size of the cost: the drag of the cost for each year, and the multiple of the cost at which the annual return
/// falls to the equal-weight line (None if it never does, or if the strategy is not above the line at 0x).
pub fn cost_note(zero: &Report, one: &Report, ew_annual: f64) -> String {
    let drag = zero.fund.annual - one.fund.annual;
    let years = one.years();
    let turnover = if years > 0.0 { one.turnover_total / years } else { f64::NAN };
    let breakeven = ((zero.fund.annual - ew_annual) / drag).is_finite().then(|| (zero.fund.annual - ew_annual) / drag).filter(|k| *k > 0.0);
    let be = breakeven.map_or("no break-even".to_string(), |k| format!("the excess over the equal-weight line of the list is zero at about {k:.0}x the cost"));
    format!("cost note: turnover {turnover:.2} a year, the cost takes {:.2}% a year at 1x; {be}. If this multiple is large, 2x is not a stress for this strategy.", 100.0 * drag)
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

/// The factor of the closes from the poison day on. It differs by ticker: a uniform factor keeps a rank. One ticker in four
/// gets 1000, so one huge return enters the beta and the momentum of a strategy that reads past the date.
pub fn price_factor(ticker: &str) -> f64 {
    let h = tick_hash(ticker);
    if h % 4 == 2 {
        1000.0
    } else {
        1.5 + 0.5 * ((h >> 8) % 3) as f64
    }
}

/// The factor of the facts filed from the poison day on. It differs by ticker.
pub fn fact_factor(ticker: &str) -> f64 {
    1.0 + 400.0 * (1 + (tick_hash(ticker) >> 16) % 11) as f64
}

/// A series (ISO date, close) with each close on `day` (YYYY-MM-DD) or later multiplied by the factor of the ticker.
pub fn poison_series(series: &[(String, f64)], day: &str, ticker: &str) -> Vec<(String, f64)> {
    let f = price_factor(ticker);
    series.iter().map(|(d, c)| (d.clone(), if d.get(..10).unwrap_or("") >= day { c * f } else { *c })).collect()
}

/// Each fact filed on `day` (YYYY-MM-DD) or later: its value multiplied by `factor`.
pub fn poison_facts(facts: &mut CompanyFacts, day: &str, factor: f64) {
    for f in facts.facts.values_mut().flat_map(|ns| ns.values_mut()).flat_map(|c| c.units.values_mut()).flatten() {
        if f.filed.as_deref().and_then(|d| d.get(..10)).is_some_and(|d| d >= day) {
            f.val = f.val.map(|v| v * factor);
        }
    }
}

/// Compare the run and the poison run. The poison day `through` is a rebalance day; the poison run changed each close and each
/// fact from that day on. The target weights of each rebalance up to `through` (included) must be equal: the strategy decides
/// on `through` with the closes before it. The trade list is compared for the days before `through` (the trade of that day is at
/// its own close, which the poison changed). Gives the number of rebalances, or the first difference.
pub fn poison_compare(clean: &Report, poisoned: &Report, through: i64) -> Result<usize, String> {
    let early = |r: &Report| r.targets.iter().filter(|(d, _)| *d <= through).cloned().collect::<Vec<_>>();
    let (a, b) = (early(clean), early(poisoned));
    if !a.iter().any(|(d, _)| *d == through) {
        return Err(format!("{} is not a rebalance day of the run: the poison day must be one", mapcheck::dates::format(through)));
    }
    for i in 0..a.len().max(b.len()) {
        match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) if x == y => {}
            (Some((d, x)), Some((_, y))) => {
                let (mx, my): (BTreeMap<_, _>, BTreeMap<_, _>) = (x.iter().cloned().collect(), y.iter().cloned().collect());
                let t = mx.keys().chain(my.keys()).find(|k| mx.get(*k) != my.get(*k)).cloned().unwrap_or_default();
                return Err(format!("rebalance {}: target weight of {t} is {:?} in the run and {:?} in the poison run", mapcheck::dates::format(*d), mx.get(&t), my.get(&t)));
            }
            _ => return Err(format!("the number of rebalances up to the poison day differs: {} and {}", a.len(), b.len())),
        }
    }
    let (ta, tb) = (trades_csv(&clean.trades, Some(through)), trades_csv(&poisoned.trades, Some(through)));
    if ta != tb {
        let line = ta.lines().zip(tb.lines()).find(|(x, y)| x != y).map_or_else(|| format!("{} and {} lines", ta.lines().count(), tb.lines().count()), |(x, y)| format!("'{x}' against '{y}'"));
        return Err(format!("the trade list before the poison day differs: {line}"));
    }
    Ok(a.len())
}

// ---- the run log ----

/// One run file (a run of the same id or of the family).
#[derive(Debug, Clone)]
pub struct RunRecord {
    pub id: String,
    /// The path from the repository root.
    pub path: String,
    /// The content (None if it cannot be read: the run is counted all the same).
    pub value: Option<Value>,
}

/// The run files of the working tree and of the whole git history (all branches, also the files that were deleted). A deleted
/// file is a hidden variant: it stays in the count.
/// Whether the git history can be read in full. None: it can. Some(reason): the counts of the runs and the earlier reads of a
/// window could be too low (git fails, or the clone is shallow). A hypothesis run is refused then; an exploratory read gets a warning.
pub fn history_state(repo: &Path) -> Option<String> {
    match git(repo, &["rev-parse", "--is-shallow-repository"]) {
        None => return Some("git cannot read this repository".to_string()),
        Some(s) if s == "true" => return Some("the clone is shallow (get the full history first: `git fetch --unshallow origin`, or a bounded `git fetch --depth=100000 origin main`)".to_string()),
        Some(_) => {}
    }
    git_bytes(repo, &["log", "--all", "--name-only", "--format=", "--", "trading/runs"]).is_none().then(|| "git log --all failed".to_string())
}

/// The run files (not the window-read files).
pub fn list_runs(repo: &Path) -> Vec<RunRecord> {
    list_files(repo).into_iter().filter(|r| !is_window_read(&r.path)).collect()
}

/// The window-read files: a read of a window by an exploratory run. It is a read, not a variant.
pub fn list_window_reads(repo: &Path) -> Vec<RunRecord> {
    list_files(repo).into_iter().filter(|r| is_window_read(&r.path)).collect()
}

fn is_window_read(path: &str) -> bool {
    path.rsplit('/').next().is_some_and(|n| n.starts_with("window-read-"))
}

fn list_files(repo: &Path) -> Vec<RunRecord> {
    let mut paths: BTreeSet<String> = BTreeSet::new();
    if let Ok(dirs) = std::fs::read_dir(repo.join("trading").join("runs")) {
        for d in dirs.flatten().filter(|d| d.path().is_dir()) {
            let id = d.file_name().to_string_lossy().into_owned();
            for f in std::fs::read_dir(d.path()).into_iter().flatten().flatten() {
                let name = f.file_name().to_string_lossy().into_owned();
                if name.ends_with(".json") {
                    paths.insert(format!("trading/runs/{id}/{name}"));
                }
            }
        }
    }
    for l in git(repo, &["log", "--all", "--name-only", "--format=", "--", "trading/runs"]).unwrap_or_default().lines() {
        let l = l.trim();
        if l.starts_with("trading/runs/") && l.ends_with(".json") && l.matches('/').count() == 3 {
            paths.insert(l.to_string());
        }
    }
    paths
        .into_iter()
        .map(|path| {
            let id = path.split('/').nth(2).unwrap_or_default().to_string();
            let from_tree = std::fs::read(repo.join(&path)).ok().and_then(|b| serde_json::from_slice(&b).ok());
            let from_git = || {
                let c = git(repo, &["log", "--all", "--diff-filter=A", "-1", "--format=%H", "--", &path])?;
                serde_json::from_slice(&git_bytes(repo, &["show", &format!("{c}:{path}")])?).ok()
            };
            let value = from_tree.or_else(from_git);
            RunRecord { id, path, value }
        })
        .collect()
}

/// The variant counts of a hypothesis. `n_*` count the run files, this run included.
#[derive(Debug, Clone, PartialEq)]
pub struct Counts {
    pub n_id: usize,
    pub n_family: usize,
    pub prior_id: u32,
    /// The `prior_variants` of each hypothesis file of the family.
    pub prior_family: u32,
    /// The earlier runs of the family that read the window.
    pub earlier_reads: Vec<String>,
    /// The dates (YYYY-MM-DD) of the exploratory reads of the window (window-read files).
    pub exploratory_dates: Vec<String>,
}

impl Counts {
    /// The variants of the hypothesis: the prior variants and the runs.
    pub fn variants_id(&self) -> usize {
        self.prior_id as usize + self.n_id
    }
    pub fn variants_family(&self) -> usize {
        self.prior_family as usize + self.n_family
    }
}

/// Count the earlier runs and add this run. A run file with no readable content counts for the family and as a read of the window.
pub fn count_runs(repo: &Path, h: &Hypothesis, r: &Resolved) -> Counts {
    let mut family_of: BTreeMap<String, (String, u32)> = BTreeMap::new();
    for e in std::fs::read_dir(hypotheses_dir(repo)).into_iter().flatten().flatten() {
        if let Some(other) = std::fs::read_to_string(e.path()).ok().and_then(|t| Hypothesis::parse_lenient(&t).ok()) {
            if let Ok(res) = resolve(repo, &other) {
                family_of.insert(other.id.clone(), (res.family, other.prior_variants.unwrap_or(0)));
            }
        }
    }
    family_of.insert(h.id.clone(), (r.family.clone(), h.prior_variants.unwrap_or(0)));
    let (mut n_id, mut n_family, mut reads) = (0, 0, Vec::new());
    for rec in list_runs(repo) {
        n_id += usize::from(rec.id == h.id);
        let fam = family_of.get(&rec.id).map(|f| f.0.clone()).or_else(|| rec.value.as_ref().and_then(|v| v.get("family").or_else(|| v.get("strategy"))).and_then(Value::as_str).map(str::to_string));
        if fam.as_deref().is_some_and(|f| f != r.family) {
            continue;
        }
        n_family += 1;
        let end = rec.value.as_ref().and_then(|v| v["config"]["end"].as_str()).and_then(mapcheck::dates::parse);
        let read = match (end, &rec.value) {
            (Some(e), _) => oos_read(e, r.window_start),
            (None, Some(v)) => v["oos_read"].as_bool().unwrap_or(true),
            (None, None) => true,
        };
        if read {
            reads.push(rec.path.clone());
        }
    }
    let prior_family = family_of.values().filter(|f| f.0 == r.family).map(|f| f.1).sum();
    let mut exploratory_dates: Vec<String> = list_window_reads(repo)
        .into_iter()
        .filter(|rec| {
            let v = rec.value.as_ref();
            v.and_then(|v| v["window"]["owner"].as_str()) == Some(r.window_id.as_str()) || family_of.get(&rec.id).is_some_and(|f| f.0 == r.family)
        })
        .filter_map(|rec| rec.path.rsplit('/').next().and_then(|n| n.strip_prefix("window-read-")).and_then(|n| n.get(..8)).map(|d| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..8])))
        .collect();
    exploratory_dates.sort();
    exploratory_dates.dedup();
    Counts { n_id: n_id + 1, n_family: n_family + 1, prior_id: h.prior_variants.unwrap_or(0), prior_family, earlier_reads: reads, exploratory_dates }
}

/// Make the file `trading/runs/<id>/<stamp>.json` with the record "started". The record is there before any result. An
/// existing file is never replaced.
pub fn write_started(repo: &Path, id: &str, stamp: &str, record: &Value) -> Result<PathBuf, String> {
    use std::io::Write;
    let dir = repo.join("trading").join("runs").join(id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{stamp}.json"));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("{}: {e} (a run log is never replaced; wait one second)", path.display()))?;
    let text = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes()).and_then(|_| file.write_all(b"\n")).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Replace the record "started" of this run by the final record. A file with any other status is never replaced.
pub fn finalize(path: &Path, record: &Value) -> Result<(), String> {
    let old: Value = serde_json::from_slice(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?).map_err(|e| format!("{}: {e}", path.display()))?;
    if old["status"].as_str() != Some("started") {
        return Err(format!("{}: the status is not 'started': a finished run log is never changed", path.display()));
    }
    let tmp = path.with_extension("tmp");
    let text = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, format!("{text}\n")).and_then(|_| std::fs::rename(&tmp, path)).map_err(|e| format!("{}: {e}", path.display()))
}

/// A run with no hypothesis file must not reach the out-of-sample window of a registered hypothesis of the same strategy.
/// The registered windows that an exploratory run (no hypothesis file) reads. The run is permitted (a head that
/// does not use the harness is not slowed), but each result line says that it is not counted and which window it read.
/// One registered window that an exploratory run reads.
#[derive(Debug, Clone, PartialEq)]
pub struct ExploreRead {
    /// The hypothesis whose window is read, and the id that owns the window.
    pub id: String,
    pub window_id: String,
    pub window_start: i64,
}

pub fn exploratory_reads(repo: &Path, strategy: &str, end: i64) -> Vec<ExploreRead> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(hypotheses_dir(repo)).into_iter().flatten().flatten() {
        let Some(h) = std::fs::read_to_string(e.path()).ok().and_then(|t| Hypothesis::parse_lenient(&t).ok()) else { continue };
        if h.strategy != strategy {
            continue;
        }
        if let Ok(r) = resolve(repo, &h) {
            if oos_read(end, r.window_start) {
                out.push(ExploreRead { id: h.id.clone(), window_id: r.window_id.clone(), window_start: r.window_start });
            }
        }
    }
    out
}

/// Write the record of an exploratory read: a new file `trading/runs/<id>/window-read-<stamp>.json`. It is a read, not a variant.
/// Never replaces a file.
pub fn write_window_read(repo: &Path, read: &ExploreRead, stamp: &str, record: &Value) -> Result<PathBuf, String> {
    use std::io::Write;
    let dir = repo.join("trading").join("runs").join(&read.id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("window-read-{stamp}.json"));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("{}: {e} (a record is never replaced; wait one second)", path.display()))?;
    let text = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes()).and_then(|_| file.write_all(b"\n")).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::strategies::{vintages_of, Vintages};
    use crate::portfolio::{run, Prices, Rebalance, Series};
    use mapcheck::registry::Registry;

    const GOOD: &str = r#"
id = "value-rank-sp500-v1"
date = "2026-11-21"
hypothesis = "The top quintile by model value over price beats the equal-weight line."
strategy = "value-rank"
universe = "rust/mapcheck/universe/sp500.txt"
oos_start = "2023-01-01"
prior_variants = 4
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

    fn d(s: &str) -> i64 {
        mapcheck::dates::parse(s).unwrap()
    }

    #[test]
    fn the_hypothesis_file_is_parsed_and_checked() {
        let h = Hypothesis::parse(GOOD).unwrap();
        assert_eq!((h.id.as_str(), h.strategy.as_str(), h.oos_start.as_str(), h.parent.clone(), h.prior_variants), ("value-rank-sp500-v1", "value-rank", "2023-01-01", None, Some(4)));
        assert!(!h.oos_prior_exposure);
        assert_eq!(h.params["paths"].as_integer(), Some(300));
        assert!(Hypothesis::parse(&GOOD.replace("2023-01-01", "2023-13-45")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("value-rank-sp500-v1", "../x")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("strategy = \"value-rank\"", "strategy = \"magic\"")).is_err());
        assert_eq!(Hypothesis::parse(&GOOD.replace("strategy = \"value-rank\"", "strategy = \"value-rank-weekly\"")).unwrap().strategy, "value-rank-weekly");
        assert!(Hypothesis::parse(&GOOD.replace("paths = 300", "pathz = 300")).is_err());
        assert!(Hypothesis::parse(&format!("{GOOD}\nextra = 1")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("oos_start = \"2023-01-01\"", "")).is_err());
        // prior_variants is needed
        assert!(Hypothesis::parse(&GOOD.replace("prior_variants = 4", "")).unwrap_err().contains("prior_variants"));
        // oos_inherits must name the parent
        assert!(Hypothesis::parse(&GOOD.replace("prior_variants = 4", "prior_variants = 4\noos_inherits = \"x\"")).is_err());
        assert!(Hypothesis::parse(&GOOD.replace("prior_variants = 4", "prior_variants = 4\nstart = \"2018-13-01\"")).is_err());
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
        // start and end from the file
        let h2 = Hypothesis::parse(&GOOD.replace("prior_variants = 4", "prior_variants = 4\nstart = \"2018-01-01\"\nend = \"2024-12-31\"\nsuccess = \"x\"")).unwrap();
        let m = h2.merge_args(&[], repo).unwrap();
        assert!(m.windows(2).any(|w| w == ["--start", "2018-01-01"]) && m.windows(2).any(|w| w == ["--end", "2024-12-31"]));
        assert!(h2.merge_args(&args(&["--end", "2025-01-01"]), repo).is_err());
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
    fn the_oos_flag_is_set_when_the_end_is_on_or_after_the_start_of_the_window() {
        assert!(!oos_read(d("2022-12-30"), d("2023-01-01")));
        assert!(oos_read(d("2023-01-01"), d("2023-01-01"))); // the series of the run has a point on this day
        assert!(oos_read(d("2023-01-02"), d("2023-01-01")));
    }

    #[test]
    fn an_ancestor_without_prior_variants_is_readable_and_counts_as_zero() {
        let old = GOOD.replace("prior_variants = 4\n", "");
        assert!(Hypothesis::parse(&old).is_err());
        assert_eq!(Hypothesis::parse_lenient(&old).unwrap().prior_variants, None);
    }

    #[test]
    fn the_window_and_the_family_come_from_the_oldest_ancestor() {
        let repo = scratch("anc");
        std::fs::create_dir_all(hypotheses_dir(&repo)).unwrap();
        let put = |id: &str, extra: &str, oos: &str| {
            let t = GOOD.replace("value-rank-sp500-v1", id).replace("2023-01-01", oos).replace("prior_variants = 4", &format!("prior_variants = 4\n{extra}"));
            std::fs::write(hypotheses_dir(&repo).join(format!("{id}.toml")), t).unwrap();
        };
        put("a", "family = \"vr\"", "2023-01-01");
        put("b", "parent = \"a\"\noos_inherits = \"a\"", "2025-06-01");
        put("c", "parent = \"b\"", "2026-01-01");
        let load = |id: &str| Hypothesis::parse(&std::fs::read_to_string(hypotheses_dir(&repo).join(format!("{id}.toml"))).unwrap()).unwrap();
        let r = resolve(&repo, &load("c")).unwrap();
        assert_eq!((r.family.as_str(), r.window_start, r.window_id.as_str(), r.chain.len()), ("vr", d("2023-01-01"), "a", 3));
        let a = resolve(&repo, &load("a")).unwrap();
        assert_eq!((a.family.as_str(), a.window_id.as_str(), a.chain.len()), ("vr", "a", 1));
        // no family in the file or in the ancestors: the strategy name
        std::fs::write(hypotheses_dir(&repo).join("z.toml"), GOOD.replace("value-rank-sp500-v1", "z")).unwrap();
        assert_eq!(resolve(&repo, &load("z")).unwrap().family, "value-rank");
        // a loop is an error
        put("p", "parent = \"q\"", "2023-01-01");
        put("q", "parent = \"p\"", "2023-01-01");
        assert!(resolve(&repo, &load("p")).is_err());
        // the clean flag: an inherited window is not clean; a prior exposure is not clean; an earlier read is not clean
        let (ok, why) = oos_clean(&load("c"), &r.window_id, &[], &[], false);
        assert!(!ok && why.len() == 1, "{why:?}");
        let (ok, _) = oos_clean(&load("a"), "a", &[], &[], false);
        assert!(ok);
        let h = Hypothesis::parse(&GOOD.replace("prior_variants = 4", "prior_variants = 4\noos_prior_exposure = true")).unwrap();
        assert!(!oos_clean(&h, &h.id, &[], &[], false).0);
        assert!(!oos_clean(&load("a"), "a", &["trading/runs/a/x.json".to_string()], &[], false).0);
        assert!(!oos_clean(&load("a"), "a", &[], &[], true).0);
        let _ = std::fs::remove_dir_all(&repo);
    }

    fn git_in(dir: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false"]).args(args).output().unwrap().status.success();
        assert!(ok, "git {args:?}");
    }

    fn started(strategy: &str, end: &str) -> Value {
        json!({ "status": "started", "strategy": strategy, "config": { "end": end } })
    }

    #[test]
    fn a_run_log_is_never_overwritten_and_the_counts_include_the_files_in_the_git_history() {
        let repo = scratch("runs");
        git_in(&repo, &["init", "-q"]);
        std::fs::create_dir_all(hypotheses_dir(&repo)).unwrap();
        let h = Hypothesis::parse(GOOD).unwrap();
        std::fs::write(hypotheses_dir(&repo).join("value-rank-sp500-v1.toml"), GOOD).unwrap();
        let r = resolve(&repo, &h).unwrap();
        let c = count_runs(&repo, &h, &r);
        assert_eq!((c.n_id, c.n_family, c.variants_id(), c.variants_family()), (1, 1, 5, 5)); // 4 prior + this run
        let p1 = write_started(&repo, &h.id, "20261121T100000Z", &started("value-rank", "2022-12-31")).unwrap();
        assert!(p1.ends_with("value-rank-sp500-v1/20261121T100000Z.json"));
        // the same name: refused, and the first file keeps its text
        let before = std::fs::read_to_string(&p1).unwrap();
        assert!(write_started(&repo, &h.id, "20261121T100000Z", &json!({ "status": "started", "n": 99 })).is_err());
        assert_eq!(std::fs::read_to_string(&p1).unwrap(), before);
        // the count of the id and of the family grows; the run ended before the window: no read
        let c = count_runs(&repo, &h, &r);
        assert_eq!((c.n_id, c.n_family, c.variants_id(), c.earlier_reads.len()), (2, 2, 6, 0));
        // another run of the same family under another id, and one of another strategy
        write_started(&repo, "other", "20261121T100001Z", &started("value-rank", "2026-01-01")).unwrap();
        write_started(&repo, "mom", "20261121T100002Z", &started("momentum", "2026-01-01")).unwrap();
        let c = count_runs(&repo, &h, &r);
        assert_eq!((c.n_id, c.n_family), (2, 3)); // the unknown id 'other' counts through its strategy field; momentum does not
        assert_eq!(c.earlier_reads, ["trading/runs/other/20261121T100001Z.json"]); // it read the window (end 2026 >= 2023)
        // the final record replaces a 'started' record once; a finished log is never changed
        finalize(&p1, &json!({ "status": "done", "strategy": "value-rank" })).unwrap();
        assert!(std::fs::read_to_string(&p1).unwrap().contains("done"));
        assert!(finalize(&p1, &json!({ "status": "done", "strategy": "value-rank", "x": 1 })).is_err());
        assert!(!std::fs::read_to_string(&p1).unwrap().contains("\"x\""));
        // a deleted file that was committed is still counted
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-q", "-m", "runs"]);
        std::fs::remove_file(&p1).unwrap();
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-q", "-m", "delete a run"]);
        assert!(!p1.exists());
        let c = count_runs(&repo, &h, &r);
        assert_eq!((c.n_id, c.n_family), (2, 3), "the deleted run file is a hidden variant: it stays in the count");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn an_exploratory_run_that_reaches_a_registered_window_is_named() {
        let repo = scratch("explore");
        std::fs::create_dir_all(hypotheses_dir(&repo)).unwrap();
        std::fs::write(hypotheses_dir(&repo).join("value-rank-sp500-v1.toml"), GOOD).unwrap();
        assert!(exploratory_reads(&repo, "value-rank", d("2022-12-31")).is_empty());
        let e = exploratory_reads(&repo, "value-rank", d("2023-01-01"));
        assert_eq!(e, vec![ExploreRead { id: "value-rank-sp500-v1".into(), window_id: "value-rank-sp500-v1".into(), window_start: d("2023-01-01") }]);
        assert!(exploratory_reads(&repo, "momentum", d("2026-01-01")).is_empty()); // another strategy
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn an_incomplete_git_history_is_found() {
        // no git repository
        let plain = scratch("nogit");
        assert!(history_state(&plain).is_some());
        // a complete repository
        let src = scratch("hist");
        git_in(&src, &["init", "-q"]);
        std::fs::write(src.join("a"), "1").unwrap();
        git_in(&src, &["add", "-A"]);
        git_in(&src, &["commit", "-q", "-m", "one"]);
        std::fs::write(src.join("a"), "2").unwrap();
        git_in(&src, &["commit", "-q", "-am", "two"]);
        assert_eq!(history_state(&src), None);
        // a shallow clone: the history is not complete
        let shallow = scratch("shallow");
        let _ = std::fs::remove_dir_all(&shallow);
        let ok = Command::new("git").args(["clone", "-q", "--depth", "1", &format!("file://{}", src.display())]).arg(&shallow).output().unwrap().status.success();
        assert!(ok);
        let why = history_state(&shallow).expect("a shallow clone must be reported");
        assert!(why.starts_with("the clone is shallow") && why.contains("--unshallow"), "{why}");
        for d in [plain, src, shallow] {
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    #[test]
    fn a_child_must_stay_in_the_family_of_its_parent() {
        let repo = scratch("fam");
        std::fs::create_dir_all(hypotheses_dir(&repo)).unwrap();
        let put = |id: &str, extra: &str| {
            let t = GOOD.replace("value-rank-sp500-v1", id).replace("prior_variants = 4", &format!("prior_variants = 4\n{extra}"));
            std::fs::write(hypotheses_dir(&repo).join(format!("{id}.toml")), t).unwrap();
        };
        put("a", "family = \"one\"");
        put("b", "parent = \"a\"\nfamily = \"two\"");
        put("c", "parent = \"a\"\nfamily = \"one\"");
        let load = |id: &str| Hypothesis::parse(&std::fs::read_to_string(hypotheses_dir(&repo).join(format!("{id}.toml"))).unwrap()).unwrap();
        assert!(resolve(&repo, &load("b")).unwrap_err().contains("differs"));
        assert_eq!(resolve(&repo, &load("c")).unwrap().family, "one");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn an_exploratory_read_is_a_record_that_makes_the_window_not_clean_and_is_not_a_variant() {
        let repo = scratch("wread");
        git_in(&repo, &["init", "-q"]);
        std::fs::create_dir_all(hypotheses_dir(&repo)).unwrap();
        std::fs::write(hypotheses_dir(&repo).join("value-rank-sp500-v1.toml"), GOOD).unwrap();
        let h = Hypothesis::parse(GOOD).unwrap();
        let r = resolve(&repo, &h).unwrap();
        let c0 = count_runs(&repo, &h, &r);
        assert!(c0.exploratory_dates.is_empty() && oos_clean(&h, &h.id, &c0.earlier_reads, &c0.exploratory_dates, false).0);
        let read = exploratory_reads(&repo, "value-rank", d("2023-03-31")).remove(0);
        let rec = json!({ "kind": "window-read", "run_time_utc": "20261121T050000Z", "window": { "owner": read.window_id } });
        let p = write_window_read(&repo, &read, "20261121T050000Z", &rec).unwrap();
        assert!(p.ends_with("value-rank-sp500-v1/window-read-20261121T050000Z.json"));
        // never replaced
        let before = std::fs::read_to_string(&p).unwrap();
        assert!(write_window_read(&repo, &read, "20261121T050000Z", &json!({ "kind": "other" })).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), before);
        // not a variant: the counts are as before; it is a read: the window is not clean, with the reason
        let c1 = count_runs(&repo, &h, &r);
        assert_eq!((c1.n_id, c1.n_family, c1.variants_id(), c1.variants_family()), (c0.n_id, c0.n_family, c0.variants_id(), c0.variants_family()));
        assert!(list_runs(&repo).is_empty() && list_window_reads(&repo).len() == 1);
        assert_eq!(c1.exploratory_dates, ["2026-11-21"]);
        let (clean, why) = oos_clean(&h, &h.id, &c1.earlier_reads, &c1.exploratory_dates, false);
        assert!(!clean && why == ["not clean (exploratory read on 2026-11-21)"], "{why:?}");
        // a deleted window-read file is still found in the git history
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-q", "-m", "w"]);
        std::fs::remove_file(&p).unwrap();
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-q", "-m", "delete"]);
        assert_eq!(count_runs(&repo, &h, &r).exploratory_dates, ["2026-11-21"]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn paths_in_the_log_are_relative_and_the_store_has_no_path() {
        let repo = scratch("rel");
        std::fs::create_dir_all(repo.join("trading")).unwrap();
        let inside = repo.join("trading").join("x.toml");
        std::fs::write(&inside, "").unwrap();
        let a = args(&["--hypothesis", &inside.to_string_lossy(), "--cache", "/home/user/fa-data", "--trades", "/tmp/t.csv", "--start", "2018-01-01"]);
        let l = log_args(&repo, &a);
        assert_eq!(l, ["--hypothesis", "trading/x.toml", "--cache", "<store>", "--trades", "<path outside the repository>", "--start", "2018-01-01"]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn a_hypothesis_file_that_is_not_committed_or_not_equal_to_head_is_refused() {
        let repo = scratch("git");
        git_in(&repo, &["init", "-q"]);
        let f = repo.join("h.toml");
        std::fs::write(&f, GOOD).unwrap();
        assert!(prove_committed(&repo, &f).unwrap_err().contains("not committed"));
        git_in(&repo, &["add", "h.toml"]);
        assert!(prove_committed(&repo, &f).unwrap_err().contains("not committed")); // added, not committed
        git_in(&repo, &["commit", "-q", "-m", "x"]);
        let p = prove_committed(&repo, &f).unwrap();
        assert_eq!((p.path.as_str(), p.sha256.as_str(), p.text.as_str()), ("h.toml", sha256_hex(GOOD.as_bytes()).as_str(), GOOD));
        assert_eq!(p.commit.len(), 40);
        assert!(p.commit_time.contains('T'));
        std::fs::write(&f, format!("{GOOD}\n# changed")).unwrap();
        assert!(prove_committed(&repo, &f).unwrap_err().contains("change"));
        // skip-worktree hides the change from `git status`: the bytes of HEAD are compared all the same
        git_in(&repo, &["update-index", "--skip-worktree", "h.toml"]);
        assert!(prove_committed(&repo, &f).unwrap_err().contains("change"));
        let _ = std::fs::remove_dir_all(&repo);
    }

    // ---- the cost levels on a fixture ----

    fn series(n: i64, f: impl Fn(i64) -> f64) -> Series {
        let d0 = d("2020-01-27");
        (0..n).map(|i| (d0 + i, f(i))).collect()
    }

    /// Alternates between A and B on each rebalance.
    struct Switch(std::sync::atomic::AtomicUsize);
    impl Strategy for Switch {
        fn name(&self) -> &str {
            "switch"
        }
        fn weights(&self, _: &View, _: &[String]) -> Result<Vec<(String, f64)>, String> {
            let i = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![(if i.is_multiple_of(2) { "A" } else { "B" }.to_string(), 1.0)])
        }
    }

    fn fixture() -> (Prices, Vec<String>, Config) {
        let items = vec![("SPY", series(200, |_| 50.0)), ("A", series(200, |i| 100.0 + i as f64)), ("B", series(200, |i| 100.0 + 0.5 * i as f64))];
        let d0 = d("2020-01-27");
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
        let note = cost_note(&zero, &one, one.fund.annual - 0.01);
        assert!(note.starts_with("cost note: turnover") && note.contains("x the cost"), "{note}");
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

    /// A strategy that reads the close `offset` days from the rebalance day in the whole series (a look-ahead if the offset is 0
    /// or more; honest at -1): it holds the ticker with the highest close on that day.
    struct Peek<'a>(&'a BTreeMap<String, Vec<(String, f64)>>, i64);
    impl Strategy for Peek<'_> {
        fn name(&self) -> &str {
            "peek"
        }
        fn weights(&self, v: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
            let day = mapcheck::dates::format(v.date() + self.1);
            let at = |t: &String| self.0[t].iter().find(|(d, _)| *d == day).map_or(0.0, |x| x.1);
            let best = universe.iter().max_by(|a, b| at(a).total_cmp(&at(b))).unwrap();
            Ok(vec![(best.clone(), 1.0)])
        }
    }

    #[test]
    fn the_poison_check_fails_for_a_look_ahead_of_one_day_and_of_twenty_days_and_passes_for_an_honest_strategy() {
        let raw: BTreeMap<String, Vec<(String, f64)>> = [("SPY", 50.0, 0.0), ("A", 100.0, 1.0), ("B", 100.0, 0.7), ("C", 100.0, 0.4), ("D", 100.0, 0.1)]
            .iter()
            .map(|(t, base, slope)| (t.to_string(), series(200, |i| base + slope * i as f64).iter().map(|(d, c)| (mapcheck::dates::format(*d), *c)).collect()))
            .collect();
        let day = "2020-04-01"; // a rebalance day (the first day of the month)
        let through = d(day);
        let poisoned: BTreeMap<String, Vec<(String, f64)>> = raw.iter().map(|(t, s)| (t.clone(), poison_series(s, day, t))).collect();
        let u: Vec<String> = ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect();
        let cfg = Config { start: d("2020-01-27"), end: through, rebalance: Rebalance::Monthly, cost_bp: 10.0 };
        // the offset -1 is the last close before the day: honest. 0 is the close of the day, 1 the day after, 20 twenty days after.
        for (offset, ok) in [(-1, true), (0, false), (1, false), (20, false)] {
            let a = run(&Prices::from_raw(&raw), &u, &Peek(&raw, offset), &cfg).unwrap();
            let b = run(&Prices::from_raw(&poisoned), &u, &Peek(&poisoned, offset), &cfg).unwrap();
            let r = poison_compare(&a, &b, through);
            assert_eq!(r.is_ok(), ok, "offset {offset}: {r:?}");
            if ok {
                assert_eq!(r, Ok(4)); // Jan 27, Feb 1, Mar 1, Apr 1: the day itself is compared
            } else {
                assert!(r.unwrap_err().contains("target weight"), "offset {offset}");
            }
        }
        // a poison day that is not a rebalance day is an error, not a pass
        let a = run(&Prices::from_raw(&raw), &u, &Peek(&raw, -1), &cfg).unwrap();
        assert!(poison_compare(&a, &a, through + 1).is_err());
    }

    /// Holds the ticker with the highest revenue of the latest fiscal year in the vintage known `offset` days from the day.
    struct FactPeek(BTreeMap<String, Vintages>, i64);
    impl Strategy for FactPeek {
        fn name(&self) -> &str {
            "fact-peek"
        }
        fn weights(&self, v: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
            let rev = |t: &String| self.0[t].known(v.date() + self.1).last().and_then(|y| y.revenue).unwrap_or(0.0);
            Ok(vec![(universe.iter().max_by(|a, b| rev(a).total_cmp(&rev(b))).unwrap().clone(), 1.0)])
        }
    }

    #[test]
    fn the_poison_check_fails_for_a_look_ahead_in_the_facts_of_twenty_days() {
        let reg = Registry::embedded().unwrap();
        let day = "2021-02-01"; // a rebalance day; the 10-K of 2020 is filed on 2021-02-19, 18 days later
        let mut names: Vec<String> = (0..8).map(|i| format!("N{i}")).collect();
        names.sort_by(|a, b| fact_factor(b).total_cmp(&fact_factor(a)));
        let (hi, lo) = (fact_factor(&names[0]), fact_factor(&names[7]));
        assert!(hi > 1.2 * lo, "the test needs different factors: {hi} {lo}");
        // revenue of 2020: the ticker with the highest factor has the lowest revenue; the poisoned ranking is the factor ranking
        let json = |i: usize| {
            format!(
                r#"{{"facts":{{"us-gaap":{{"Revenues":{{"units":{{"USD":[
            {{"start":"2019-01-01","end":"2019-12-31","val":100,"form":"10-K","filed":"2020-02-20"}},
            {{"start":"2020-01-01","end":"2020-12-31","val":{},"form":"10-K","filed":"2021-02-19"}}]}}}}}}}}}}"#,
                100 + i
            )
        };
        let years = |poison: bool| -> BTreeMap<String, Vintages> {
            names
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let mut facts: CompanyFacts = serde_json::from_str(&json(i)).unwrap();
                    if poison {
                        poison_facts(&mut facts, day, fact_factor(t));
                    }
                    (t.clone(), vintages_of(&reg, t, &facts, 0, &[], "2026-11-21").unwrap().1)
                })
                .collect()
        };
        let items: Vec<(String, Series)> = names.iter().map(|t| (t.clone(), series(150, |_| 10.0))).chain([("SPY".to_string(), series(150, |_| 50.0))]).collect();
        let shift = |s: Series| -> Series { s.into_iter().map(|(x, c)| (x - d("2020-01-27") + d("2020-12-01"), c)).collect() };
        let prices = Prices(items.into_iter().map(|(t, s)| (t, shift(s))).collect());
        let cfg = Config { start: d("2020-12-01"), end: d(day), rebalance: Rebalance::Monthly, cost_bp: 10.0 };
        for (offset, ok) in [(0, true), (20, false)] {
            let a = run(&prices, &names, &FactPeek(years(false), offset), &cfg).unwrap();
            let b = run(&prices, &names, &FactPeek(years(true), offset), &cfg).unwrap();
            let r = poison_compare(&a, &b, d(day));
            assert_eq!(r.is_ok(), ok, "offset {offset}: {r:?}");
        }
    }

    #[test]
    fn poison_factors_differ_by_ticker_and_poison_the_day_itself_and_later() {
        let f: BTreeSet<u64> = ["AAPL", "IBM", "JNJ", "MSFT", "XOM", "KO", "PG", "T"].iter().map(|t| fact_factor(t).to_bits()).collect();
        assert!(f.len() >= 5);
        let s = vec![("2020-03-31".to_string(), 10.0), ("2020-04-01".to_string(), 10.0), ("2020-04-02".to_string(), 10.0)];
        let p = poison_series(&s, "2020-04-01", "IBM");
        assert_eq!(p[0].1, 10.0);
        assert_eq!((p[1].1, p[2].1), (10.0 * price_factor("IBM"), 10.0 * price_factor("IBM")));
        assert!(price_factor("IBM") > 1.0);
        let mut facts: CompanyFacts = serde_json::from_str(
            r#"{"facts":{"us-gaap":{"Revenues":{"units":{"USD":[
            {"end":"2019-12-31","val":100,"form":"10-K","filed":"2020-02-20"},
            {"end":"2020-12-31","val":120,"form":"10-K","filed":"2020-12-31"},
            {"end":"2020-12-31","val":130,"form":"10-K","filed":"2021-02-19"},
            {"end":"2020-12-31","val":140,"form":"10-K"}]}}}}}"#,
        )
        .unwrap();
        poison_facts(&mut facts, "2020-12-31", 1000.0);
        let v: Vec<Option<f64>> = facts.facts["us-gaap"]["Revenues"].units["USD"].iter().map(|f| f.val).collect();
        assert_eq!(v, [Some(100.0), Some(120_000.0), Some(130_000.0), Some(140.0)]);
    }
}
