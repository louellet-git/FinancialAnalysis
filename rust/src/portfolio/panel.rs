//! `fa portfolio panel`: the signal panel of the trade-level backtest (ticket `T-20261121-trade-level-backtest`, part 1).
//!
//! Each Friday of the weekly calendar (`Rebalance::Weekly`), each ticker: the automatic model of `ValueRank` (the same
//! function `ValueRank::model_row`: the vintage filed before the date, the split basis through `share_basis`, the curve
//! and the beta of the day before, the freshness rules) gives P10, P25, P50, P75, P90 and the mean per share. The panel
//! keeps these with the close of the day before, the filing accession and the FF12 sector. The model of date D reads
//! nothing from D or later: the strategies tests of `strategies.rs` hold the poison and the truncation tests of the panel.
//!
//! File: one header line that starts with `# fa-panel v1` (key=value words), then a CSV. A reader (`read_panel`,
//! `check_fresh`) refuses a file whose header does not match the code, the store and the split table of the run.

use super::cli::{check_options, day_of, load_inputs, load_vr_parts, opt, sector_map, table_file_of, value_rank_years, write_new, Inputs};
use super::harness::{code_state, git, sha256_hex, store_state};
use super::strategies::{Pick, ValueRank};
use super::{rebalance_days, Prices, Rebalance, View};
use crate::dcf::auto::index_from_cache;
use crate::dcf::auto_returns::{beta_reset, beta_warning, RetParams};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa portfolio panel --tickers-file F --start D --end D [OPTIONS]
  The signal panel of the trade-level backtest. Each Friday (the weekly days of the backtest), each ticker: the automatic
  model with the point-in-time inputs of value-rank (the SEC facts filed before the date, split-corrected share counts on the
  basis of the closes of the store, the curve and the beta of the day before, the freshness rules of value-rank: the last
  fiscal year at most 550 days old, a close at most 7 days old, P50 above 0). One row for each scored ticker and date: date,
  ticker, the close of the day before and its date, P10, P25, P50, P75, P90, the mean, the filing accession, the FF12 sector.
  The rows are computed once, in parallel; `fa portfolio trades` reads the file. No network.
  --tickers-file F   the universe (the text is hashed into the header)
  --cache DIR        the data store (default <repo>/data/raw). The command only reads it.
  --start D --end D  first and last day, YYYY-MM-DD (--end default: the last SPY day)
  --table FILE       parameter table (default analysis/priors/process_priors_2026-11-19.toml)
  --paths N (300)    paths of each model (100 or more)        --seed S (7)
  --out FILE         the panel file (default <repo>/data/processed/panel_<start>_<end>_<8 characters of the tickers hash>.csv;
                     the file is not in the store, and a file is never replaced)
  The first line of the file holds the code commit, the store commit, the hash and the commit of the split table, the
  table hash and the options. `fa portfolio trades` refuses a panel whose code, store or split table is not the current
  one: build it again. The command prints the time for 1,000 models.
  Survivorship: the list of tickers is today's list.";

/// One row of the panel: one ticker on one date.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelRow {
    pub date: i64,
    pub ticker: String,
    /// The last close before `date` and its day.
    pub close_day: i64,
    pub close: f64,
    /// P10, P25, P50, P75, P90 of the value for each share.
    pub q: [f64; 5],
    /// The mean of the finite paths (NaN if none).
    pub mean: f64,
    pub accession: String,
    pub sector: String,
}

const COLUMNS: &str = "date,ticker,close_day,close,p10,p25,p50,p75,p90,mean,accession,sector";

/// The header line of a panel or a trades file: `key=value` words (no space in a value).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header(pub BTreeMap<String, String>);

impl Header {
    pub fn line(&self, kind: &str) -> String {
        let words: Vec<String> = self.0.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("# {kind} v1 {}", words.join(" "))
    }

    pub fn parse(line: &str, kind: &str) -> Result<Header, String> {
        let rest = line.strip_prefix(&format!("# {kind} v1 ")).ok_or_else(|| format!("the first line is not a '{kind} v1' header"))?;
        let mut m = BTreeMap::new();
        for w in rest.split_whitespace() {
            let (k, v) = w.split_once('=').ok_or_else(|| format!("header word '{w}' is not key=value"))?;
            m.insert(k.to_string(), v.to_string());
        }
        Ok(Header(m))
    }

    pub fn get(&self, k: &str) -> &str {
        self.0.get(k).map_or("", String::as_str)
    }
}

/// What the code, the store and the split table are now.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    pub code: String,
    pub code_dirty: bool,
    pub store: String,
    pub splits_sha256: String,
    pub splits_commit: String,
}

fn text_or_none(x: Option<String>) -> String {
    x.filter(|s| !s.is_empty()).unwrap_or_else(|| "none".to_string())
}

impl Provenance {
    /// The state of the repository `repo` (the code), the cache (the store) and `<cache>/_corporate/splits.csv`.
    pub fn current(repo: &Path, cache: &Path) -> Provenance {
        let (code, dirty) = code_state(repo);
        let splits = cache.join("_corporate").join("splits.csv");
        Provenance {
            code: text_or_none(code),
            code_dirty: dirty,
            store: text_or_none(store_state(cache).0),
            splits_sha256: std::fs::read(&splits).map(|b| sha256_hex(&b)).unwrap_or_else(|_| "none".into()),
            splits_commit: text_or_none(git(cache, &["log", "-1", "--format=%H", "--", "_corporate/splits.csv"])),
        }
    }

    pub fn put(&self, h: &mut Header) {
        for (k, v) in [("code", &self.code), ("store", &self.store), ("splits_sha256", &self.splits_sha256), ("splits_commit", &self.splits_commit)] {
            h.0.insert(k.to_string(), v.clone());
        }
        h.0.insert("code_dirty".into(), self.code_dirty.to_string());
    }
}

/// A reader refuses a header that does not match the code, the store and the split table of now (`fa portfolio panel` again).
pub fn check_fresh(h: &Header, now: &Provenance) -> Result<(), String> {
    let mut bad = Vec::new();
    for (k, v) in [("code", &now.code), ("store", &now.store), ("splits_sha256", &now.splits_sha256), ("splits_commit", &now.splits_commit)] {
        if h.get(k) != v {
            bad.push(format!("{k}: file {}, now {v}", h.get(k)));
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("the panel does not match the code, the store or the split table of this run ({}): build it again with fa portfolio panel", bad.join("; ")))
    }
}

/// The rows of the panel for the dates and the tickers, sorted by date and ticker. Parallel over the pairs (date, ticker).
/// The model is `ValueRank::model_row`, the one function under the score of the strategy.
pub fn build_rows(vr: &ValueRank, prices: &Prices, universe: &[String], dates: &[i64], sectors: &BTreeMap<String, String>) -> Vec<PanelRow> {
    let tasks: Vec<(i64, &String)> = dates.iter().flat_map(|d| universe.iter().map(move |t| (*d, t))).collect();
    tasks
        .par_iter()
        .filter_map(|(d, t)| {
            let r = vr.model_row(&View::new(*d, prices), t)?;
            Some(PanelRow {
                date: *d,
                ticker: (*t).clone(),
                close_day: r.close_day,
                close: r.close,
                q: r.dist.q,
                mean: r.dist.mean,
                accession: r.accession,
                sector: sectors.get(*t).cloned().unwrap_or_else(|| "Other".into()),
            })
        })
        .collect()
}

/// The CSV text of the rows (after the header line). Numbers are written in the shortest form that reads back exactly.
pub fn rows_csv(rows: &[PanelRow]) -> String {
    let mut s = String::with_capacity(rows.len() * 120);
    let _ = writeln!(s, "{COLUMNS}");
    for r in rows {
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            mapcheck::dates::format(r.date),
            r.ticker,
            mapcheck::dates::format(r.close_day),
            r.close,
            r.q[0],
            r.q[1],
            r.q[2],
            r.q[3],
            r.q[4],
            if r.mean.is_finite() { r.mean.to_string() } else { String::new() },
            r.accession,
            r.sector
        );
    }
    s
}

/// Read a panel text: the header, and the rows. The header says the number of rows: a cut file is an error.
pub fn read_panel(text: &str) -> Result<(Header, Vec<PanelRow>), String> {
    let mut lines = text.lines();
    let header = Header::parse(lines.next().unwrap_or(""), "fa-panel")?;
    if lines.next() != Some(COLUMNS) {
        return Err("the panel columns are not the expected ones".into());
    }
    let num = |w: &str, what: &str| w.parse::<f64>().map_err(|_| format!("panel: bad {what} '{w}'"));
    let day = |w: &str| mapcheck::dates::parse(w).ok_or_else(|| format!("panel: bad date '{w}'"));
    let mut rows = Vec::new();
    for l in lines {
        let w: Vec<&str> = l.split(',').collect();
        if w.len() != 12 {
            return Err(format!("panel: a row has {} fields, not 12: '{l}'", w.len()));
        }
        rows.push(PanelRow {
            date: day(w[0])?,
            ticker: w[1].to_string(),
            close_day: day(w[2])?,
            close: num(w[3], "close")?,
            q: [num(w[4], "p10")?, num(w[5], "p25")?, num(w[6], "p50")?, num(w[7], "p75")?, num(w[8], "p90")?],
            mean: if w[9].is_empty() { f64::NAN } else { num(w[9], "mean")? },
            accession: w[10].to_string(),
            sector: w[11].to_string(),
        });
    }
    if header.get("n_rows") != rows.len().to_string() {
        return Err(format!("the panel header says {} rows, the file has {}: the file is cut", header.get("n_rows"), rows.len()));
    }
    Ok((header, rows))
}

const KNOWN: [&str; 8] = ["--tickers-file", "--cache", "--start", "--end", "--table", "--paths", "--seed", "--out"];

/// `fa portfolio panel`.
pub fn command(repo: &Path, args: &[String]) -> Result<(), String> {
    check_options(args, &KNOWN)?;
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let num = |name: &str, default: u64| -> Result<u64, String> { opt(args, name).map_or(Ok(default), |v| v.parse().map_err(|_| format!("{name} '{v}': a whole number"))) };
    let paths = num("--paths", 300)? as usize;
    if paths < 100 {
        return Err(format!("--paths {paths}: 100 or more"));
    }
    let seed = num("--seed", 7)?;
    let start = day_of(args, "--start")?.ok_or("--start is needed")?;
    if opt(args, "--tickers-file").is_none() {
        return Err("--tickers-file is needed (the panel of a whole cache is not a default)".into());
    }
    let t0 = std::time::Instant::now();
    let Inputs { tickers, raw, prices, universe_text } = load_inputs(args, &cache)?;
    let universe_text = universe_text.unwrap_or_default();
    let spy_last = prices.0["SPY"].last().map(|x| x.0).ok_or("SPY has no prices")?;
    let end = day_of(args, "--end")?.unwrap_or(spy_last);
    let tickers_sha = sha256_hex(universe_text.as_bytes());
    let out_path = match opt(args, "--out") {
        Some(p) => PathBuf::from(p),
        None => repo.join("data").join("processed").join(format!("panel_{}_{}_{}.csv", mapcheck::dates::format(start), mapcheck::dates::format(end), &tickers_sha[..8])),
    };
    if out_path.exists() {
        return Err(format!("{}: the file exists; the command does not replace a file", out_path.display()));
    }
    let cal: Vec<i64> = prices.0["SPY"].iter().map(|x| x.0).filter(|d| *d >= start && *d <= end).collect();
    let dates = rebalance_days(&cal, start, end, Rebalance::Weekly);

    let parts = load_vr_parts(repo, args, &cache, &tickers)?;
    let (years, universe) = value_rank_years(&parts, &cache, &tickers, start, None);
    eprintln!("panel: {} of {} tickers have SEC facts with the standard template; {} Fridays", universe.len(), tickers.len(), dates.len());
    let sectors = sector_map(&cache, &universe, Some("sector"))?.ok_or("the sector of each ticker is needed")?;
    let index = index_from_cache(&cache);
    beta_reset();
    let params = RetParams { table: &parts.table, fade: &parts.fade, rules: &parts.rules, curves: parts.curves.as_ref(), index: None, lag: 100, paths, seed };
    let vr = ValueRank { years, series: &raw, index: index.as_deref(), params, scored: Default::default(), signals: Default::default(), sectors: Some(sectors.clone()), pick: Pick::Top };
    let t_model = std::time::Instant::now();
    let rows = build_rows(&vr, &prices, &universe, &dates, &sectors);
    let secs = t_model.elapsed().as_secs_f64();

    let prov = Provenance::current(repo, &cache);
    let mut h = Header::default();
    prov.put(&mut h);
    let table_file = table_file_of(repo, args);
    h.0.insert("table_sha256".into(), std::fs::read(&table_file).map(|b| sha256_hex(&b)).unwrap_or_else(|_| "none".into()));
    for (k, v) in [
        ("paths", paths.to_string()),
        ("seed", seed.to_string()),
        ("start", mapcheck::dates::format(start)),
        ("end", mapcheck::dates::format(end)),
        ("tickers_sha256", tickers_sha),
        ("n_tickers", universe.len().to_string()),
        ("n_dates", dates.len().to_string()),
        ("n_rows", rows.len().to_string()),
    ] {
        h.0.insert(k.to_string(), v);
    }
    if let Some(dir) = out_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    write_new(&out_path.to_string_lossy(), &format!("{}\n{}", h.line("fa-panel"), rows_csv(&rows)))?;
    let n = rows.len().max(1) as f64;
    let beta_note = beta_warning(index.is_none()).unwrap_or_default();
    print!("{}{beta_note}", parts.split_note);
    println!(
        "panel written: {} ({} rows, {} tickers, {} dates). Models: {:.1} s for {} rows, {:.2} s for 1,000 models on {} threads (the models that refuse are in the time, not in the rows); with the loading {:.1} s.",
        out_path.display(),
        rows.len(),
        universe.len(),
        dates.len(),
        secs,
        rows.len(),
        1000.0 * secs / n,
        rayon::current_num_threads(),
        t0.elapsed().as_secs_f64()
    );
    if prov.code_dirty {
        println!("WARNING: the code has changes that are not committed (code_dirty=true in the header).");
    }
    println!("{} {}", super::harness::UNIVERSE_LABEL, "(survivorship: the result of a trade rule is too good)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<PanelRow> {
        let d = |s: &str| mapcheck::dates::parse(s).unwrap();
        vec![
            PanelRow { date: d("2020-09-04"), ticker: "A".into(), close_day: d("2020-09-03"), close: 12.3456789012345, q: [1.0, 2.5, 10.0 / 3.0, 20.0, 90.5], mean: 25.25, accession: "0000000001-20-000001".into(), sector: "P".into() },
            PanelRow { date: d("2020-09-04"), ticker: "B".into(), close_day: d("2020-09-03"), close: 7.0, q: [0.1, 0.2, 0.3, 0.4, 0.5], mean: f64::NAN, accession: String::new(), sector: "Other".into() },
        ]
    }

    fn text_of(r: &[PanelRow]) -> String {
        let mut h = Header::default();
        h.0.insert("code".into(), "abc".into());
        h.0.insert("n_rows".into(), r.len().to_string());
        format!("{}\n{}", h.line("fa-panel"), rows_csv(r))
    }

    #[test]
    fn the_panel_text_reads_back_exactly() {
        let (h, back) = read_panel(&text_of(&rows())).unwrap();
        assert_eq!(h.get("code"), "abc");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0], rows()[0]);
        assert!(back[1].mean.is_nan());
        assert_eq!((back[1].q, back[1].close), (rows()[1].q, 7.0));
    }

    #[test]
    fn a_cut_panel_or_a_file_with_no_header_is_refused() {
        let text = text_of(&rows());
        let cut: String = text.lines().take(3).collect::<Vec<_>>().join("\n");
        assert!(read_panel(&cut).unwrap_err().contains("cut"));
        assert!(read_panel("date,ticker\n").is_err());
    }

    #[test]
    fn a_reader_refuses_a_panel_of_other_code_store_or_split_table() {
        let now = Provenance { code: "c1".into(), code_dirty: false, store: "s1".into(), splits_sha256: "h1".into(), splits_commit: "k1".into() };
        let mut h = Header::default();
        now.put(&mut h);
        assert!(check_fresh(&h, &now).is_ok());
        for (key, value) in [("code", "c0"), ("store", "s0"), ("splits_sha256", "h0"), ("splits_commit", "k0")] {
            let mut bad = h.clone();
            bad.0.insert(key.into(), value.into());
            let e = check_fresh(&bad, &now).unwrap_err();
            assert!(e.contains(key) && e.contains("build it again"), "{e}");
        }
    }

    #[test]
    fn the_header_line_reads_back() {
        let mut h = Header::default();
        h.0.insert("a".into(), "1".into());
        h.0.insert("b".into(), "x-y".into());
        assert_eq!(Header::parse(&h.line("fa-panel"), "fa-panel").unwrap(), h);
        assert!(Header::parse(&h.line("fa-panel"), "fa-trades").is_err());
    }
}
