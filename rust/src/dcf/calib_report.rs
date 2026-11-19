//! `fa dcf calibrate`: the command, the text of the result, and the growth fade by starting level
//! (ticket `T-20261119-dcf-process-calibration`).

use super::calib_bands::{coverage_report, render_coverage};
use super::calibrate::{estimate_table, Ar1Params, FadeCell, FadeFile, FadeStep, ProcessPriors, ProcessTable, TableMeta, GROWTH_LIMITS};
use super::panel::{build_panel, bucket, CompanyPanel, Panel, BUCKET_LABELS};
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use std::fmt::Write;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa dcf calibrate [OPTIONS]
  Estimate the process parameters of revenue growth and of the margins from the company cache
  (an AR(1) for each series, by size bucket of revenue and by template), the growth fade of fast growers,
  and the calibration of the forecast bands. No network.
  --cache DIR        company facts (default <repo>/data/raw); a ticker folder has company_facts.json[.gz]
  --out FILE         write the table of parameters (TOML)
  --reps N (200)     resamples of the bootstrap over companies for the shock correlations
  --no-bands         skip the band calibration (step 4)
  --draws N (2000)   paths for each band of the band calibration
  --panel-out FILE   write the panel (tab separated)
  --date D (2026-11-19)  date written in the table
  --table FILE --lookup REVENUE,TEMPLATE
                     print the parameters of a company from a table (no cache needed)";

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// The size label and the template of a lookup.
fn lookup(table: &Path, spec: &str) -> Result<(), DcfError> {
    let (rev, template) = spec.split_once(',').ok_or_else(|| DcfError("--lookup needs REVENUE,TEMPLATE".into()))?;
    let revenue: f64 = rev.parse().map_err(|_| DcfError(format!("bad revenue '{rev}'")))?;
    let t = ProcessTable::load(table)?;
    let p = t.priors_for(revenue, template, None).ok_or_else(|| DcfError("no cell has enough observations".into()))?;
    println!("{}", toml::to_string_pretty(&p).map_err(|x| DcfError(x.to_string()))?);
    Ok(())
}

/// `fa dcf calibrate`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> Result<u8, DcfError> {
    if flag(args, "-h") || flag(args, "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    for (i, a) in args.iter().enumerate() {
        let known_flags = ["--no-bands"];
        let known_values = ["--cache", "--out", "--reps", "--draws", "--panel-out", "--date", "--table", "--lookup"];
        let is_value = i > 0 && known_values.contains(&args[i - 1].as_str());
        if a.starts_with("--") && !known_flags.contains(&a.as_str()) && !known_values.contains(&a.as_str()) {
            return Err(DcfError(format!("unknown option '{a}'")));
        }
        if !a.starts_with("--") && !is_value {
            return Err(DcfError(format!("unexpected word '{a}'")));
        }
    }
    if let Some(spec) = opt(args, "--lookup") {
        let table = opt(args, "--table").ok_or_else(|| DcfError("--lookup needs --table FILE".into()))?;
        lookup(Path::new(table), spec)?;
        return Ok(0);
    }
    let cache: PathBuf = opt(args, "--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"));
    let num = |name: &str, default: usize| -> Result<usize, DcfError> {
        match opt(args, name) {
            None => Ok(default),
            Some(v) => v.parse().map_err(|_| DcfError(format!("bad value for {name}: '{v}'"))),
        }
    };
    let reps = num("--reps", 200)?;
    let draws = num("--draws", 2000)?;
    let date = opt(args, "--date").unwrap_or("2026-11-19").to_string();

    let cache_text = if cache.ends_with("data/raw") { "data/raw".to_string() } else { cache.display().to_string() };
    let panel = build_panel(&cache, None).map_err(DcfError)?;
    let mut out = String::new();
    render_panel(&mut out, &panel);
    if let Some(path) = opt(args, "--panel-out") {
        std::fs::write(path, panel_tsv(&panel)).map_err(|x| DcfError(format!("{path}: {x}")))?;
        let _ = writeln!(out, "Panel written to {path}");
    }
    let t0 = std::time::Instant::now();
    let cells = estimate_table(&panel.companies, reps);
    let _ = writeln!(out, "\nEstimation: {} cells in {:.1} s", cells.len(), t0.elapsed().as_secs_f64());
    render_estimates(&mut out, &cells);
    render_fade(&mut out, &panel.companies, &cells);
    if !flag(args, "--no-bands") {
        let t1 = std::time::Instant::now();
        let report = coverage_report(&panel.companies, draws);
        let _ = writeln!(out, "\nBand calibration: {:.1} s", t1.elapsed().as_secs_f64());
        render_coverage(&mut out, &report);
    }
    out.push_str(SURVIVORSHIP);
    print!("{out}");
    if let Some(path) = opt(args, "--out") {
        let table = ProcessTable {
            meta: TableMeta {
                date,
                command: format!("fa dcf calibrate --cache {cache_text} --out {path} --reps {reps} --draws {draws}"),
                companies: panel.counts.companies,
                company_years: panel.company_years(),
                growth_observations: panel.counts.growth_obs,
                note: "AR(1) y_t = mean + phi (y_t-1 - mean) + sigma e_t. phi = within estimate with the Nickell correction. Growth: jumps above 60% removed. Margins: absolute value up to 1. Size = revenue of the year before. No sector cells (the cache has no SIC code). Survivorship: see docs/ARCHITECTURE.md.".to_string(),
            },
            cell: cells,
        };
        let fade = FadeFile { fade: fade_cells(&panel.companies, &table.cell) };
        let text = format!("{}\n{}", table.to_toml()?, fade.to_toml()?);
        std::fs::write(path, text).map_err(|x| DcfError(format!("{path}: {x}")))?;
        println!("\nTable written to {path}");
    }
    Ok(0)
}

pub const SURVIVORSHIP: &str = "
SURVIVORSHIP
  The cache has companies that exist now (the SEC ticker list: the S&P 500 and the next 500). Companies that
  failed, were bought, or were removed from the index are not in it, and the index adds a company after its rise.
  - Growth level (mean): biased UP. The missing companies have low or negative growth in their last years.
  - Growth persistence (phi) and half-life: unclear sign. The data keep the companies that stayed large: their
    low-growth years are less often followed by decline, and a decline does not end the series. Expect phi too
    high for the low side and too low for the high side (a fast grower that fell back is in the sample only if
    it stayed in the index).
  - Growth shock size (sigma) and the tails: biased DOWN. The lower tail (bankruptcy, forced sales, collapse) is
    cut off.
  - Margins: mean biased UP (a loss maker that failed is missing), sigma biased DOWN, phi biased UP (a bad
    margin that did not recover is a failure, so it left the sample).
  - Fade of fast growers: the median growth after 3 to 10 years is biased UP, since a company that fell hard
    left the sample; the P10 is biased UP the most.
  - Band coverage: the real values are inside the bands more often than they would be for a new draw of
    companies, so a coverage below 80% is a strong result and a coverage above 80% is weak evidence.
  The size of the bias is not measured here: the cache has no data of the missing companies.
";

fn render_panel(out: &mut String, p: &Panel) {
    let c = &p.counts;
    let _ = writeln!(out, "PANEL ({:.1} s)", p.seconds);
    let _ = writeln!(out, "  tickers in the cache              {:>7}", c.tickers_in_cache);
    let _ = writeln!(out, "  unreadable                        {:>7}", c.unreadable);
    let _ = writeln!(out, "  not us-gaap (no panel)            {:>7}", c.not_us_gaap);
    let _ = writeln!(out, "  no fiscal year with revenue       {:>7}", c.no_annual_revenue);
    let _ = writeln!(out, "  companies in the panel            {:>7}", c.companies);
    let _ = writeln!(out, "  fiscal years from the mapping     {:>7}", c.years_in);
    let _ = writeln!(out, "  rule 1 duplicate years removed    {:>7}", c.duplicate_years);
    let _ = writeln!(out, "  rule 2 no revenue above 0 removed {:>7}", c.no_revenue);
    let _ = writeln!(out, "  company-years in the panel        {:>7}", c.rows_out);
    let _ = writeln!(out, "  pairs of years                    {:>7}", c.pairs);
    let _ = writeln!(out, "  rule 3 missing year (gap > 380 d) {:>7}  (growth not defined)", c.gap_missing_year);
    let _ = writeln!(out, "  rule 3 short year (gap < 350 d)   {:>7}  (growth not defined)", c.gap_short_year);
    let _ = writeln!(out, "  rule 3 long year (gap > 366 d)    {:>7}  (kept, counted)", c.long_years);
    let _ = writeln!(out, "  growth observations               {:>7}", c.growth_obs);
    let _ = writeln!(out, "  rule 4 jumps (>60%, prev rev >=1B){:>7}  (removed from the AR(1) sample)", c.jumps);
    let _ = writeln!(out, "    of these, cash for businesses   {:>7}  (acquisition likely)", c.jumps_acq_likely);
    let by = p.rows_by_bucket();
    let gb = p.growth_obs_by_bucket();
    let _ = writeln!(out, "  by revenue bucket        company-years   growth obs (bucket of the year before)");
    for i in 0..5 {
        let _ = writeln!(out, "    {:<10}             {:>7}        {:>7}", BUCKET_LABELS[i], by[i], gb[i]);
    }
    let t: Vec<String> = p.companies_by_template().iter().map(|(k, n)| format!("{k} {n}")).collect();
    let _ = writeln!(out, "  templates: {}", t.join(", "));
    let _ = writeln!(out, "  sector: the company facts and the cache have no SIC code. Groups use the template and the size.");
}

fn panel_tsv(p: &Panel) -> String {
    let mut s = String::from("ticker\ttemplate\tend\tyear\trevenue\toperating_income\toperating_cash_flow\tcapex\tfcf\tsbc\tprev_ok\tgrowth\tjump\tacq_likely\n");
    let f = |v: Option<f64>| v.map_or(String::new(), |x| format!("{x}"));
    for c in &p.companies {
        for r in &c.rows {
            let _ = writeln!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                c.ticker,
                c.template,
                r.end,
                r.year,
                r.revenue,
                f(r.operating_income),
                f(r.operating_cash_flow),
                f(r.capex),
                f(r.fcf),
                f(r.sbc),
                r.prev_ok,
                f(r.growth),
                r.jump,
                r.acq_likely
            );
        }
    }
    s
}

fn pct(v: f64) -> String {
    format!("{:6.1}%", v * 100.0)
}

fn est(phi: f64, se: f64) -> String {
    format!("{phi:6.3} ({se:5.3})")
}

fn render_series(out: &mut String, title: &str, cells: &[ProcessPriors], get: &dyn Fn(&ProcessPriors) -> Option<&Ar1Params>, as_pct: bool) {
    let _ = writeln!(out, "\n{title}");
    let _ = writeln!(
        out,
        "  {:<8}{:<9}{:>7}{:>6}  {:<14}{:<14}{:<14}{:<14}{:>6}{:>9}{:>8}{:>8}",
        "size", "template", "obs", "cos", "phi pooled", "phi FE", "phi FE+Nickell", "phi AH", "HL(y)", "mean", "sd(mean)", "sigma"
    );
    let v = |x: f64| if as_pct { pct(x) } else { format!("{x:7.3}") };
    for c in cells {
        let Some(p) = get(c) else { continue };
        if !(c.template == "all" || c.template == "standard") {
            continue;
        }
        let ah = match (p.phi_ah, p.phi_ah_se) {
            (Some(a), Some(s)) => est(a, s),
            _ => "      n/a     ".to_string(),
        };
        let _ = writeln!(
            out,
            "  {:<8}{:<9}{:>7}{:>6}  {:<14}{:<14}{:<14}{:<14}{:>6.1}{:>9}{:>8}{:>8}",
            c.size,
            c.template,
            p.n_obs,
            p.n_units,
            est(p.phi_pooled, p.phi_pooled_se),
            est(p.phi_fe, p.phi_fe_se),
            est(p.phi, p.phi_se),
            ah,
            p.half_life,
            v(p.mean),
            v(p.mean_sd_units),
            v(p.sigma)
        );
    }
}

fn render_by_template(out: &mut String, title: &str, cells: &[ProcessPriors], get: &dyn Fn(&ProcessPriors) -> Option<&Ar1Params>) {
    let _ = writeln!(out, "\n{title}: by template (all sizes)");
    for c in cells.iter().filter(|c| c.size == "all") {
        if let Some(p) = get(c) {
            let _ = writeln!(
                out,
                "  {:<9} obs {:>6} cos {:>5}  phi pooled {:6.3}  FE {:6.3}  FE+Nickell {} HL {:4.1}  mean {}  sigma {}",
                c.template,
                p.n_obs,
                p.n_units,
                p.phi_pooled,
                p.phi_fe,
                est(p.phi, p.phi_se),
                p.half_life,
                pct(p.mean),
                pct(p.sigma)
            );
        }
    }
}

fn render_estimates(out: &mut String, cells: &[ProcessPriors]) {
    let _ = writeln!(out, "\nAR(1) ESTIMATES. Standard errors in brackets: robust to correlation inside a company (cluster by company).");
    let _ = writeln!(out, "  Size: revenue of the year before. phi FE+Nickell is the recommended phi. HL = half-life in years.");
    // size rows first, with the "all" size at the top
    let ordered = |cells: &[ProcessPriors]| -> Vec<ProcessPriors> {
        let mut v: Vec<ProcessPriors> = cells.to_vec();
        let key = |c: &ProcessPriors| {
            let s = if c.size == "all" { 0 } else { 1 + BUCKET_LABELS.iter().position(|l| *l == c.size).unwrap_or(9) };
            (if c.template == "all" { 0 } else { 1 }, s)
        };
        v.sort_by_key(key);
        v
    };
    let o = ordered(cells);
    render_series(out, "REVENUE GROWTH, nominal (jumps removed)", &o, &|c| c.growth.as_ref(), true);
    render_by_template(out, "REVENUE GROWTH, nominal", cells, &|c| c.growth.as_ref());
    render_series(out, "REVENUE GROWTH less the mean growth of the fiscal year (no GDP series in the cache; the year mean replaces it)", &o, &|c| c.growth_excess.as_ref(), true);
    render_series(out, "OPERATING MARGIN", &o, &|c| c.operating_margin.as_ref(), true);
    render_by_template(out, "OPERATING MARGIN", cells, &|c| c.operating_margin.as_ref());
    render_series(out, "FREE CASH FLOW MARGIN", &o, &|c| c.fcf_margin.as_ref(), true);
    render_by_template(out, "FREE CASH FLOW MARGIN", cells, &|c| c.fcf_margin.as_ref());
    let _ = writeln!(out, "\nCORRELATION of the growth shock with the margin shock (residuals of the within AR(1) models; bootstrap over companies)");
    let _ = writeln!(out, "  {:<8}{:<9}{:>22}{:>22}", "size", "template", "growth, op. margin", "growth, FCF margin");
    for c in o.iter().filter(|c| c.template == "all" || c.template == "standard") {
        let f = |r: Option<f64>, s: Option<f64>| match (r, s) {
            (Some(r), Some(s)) => format!("{r:+.3} ({s:.3})"),
            _ => "n/a".to_string(),
        };
        let _ = writeln!(
            out,
            "  {:<8}{:<9}{:>22}{:>22}",
            c.size,
            c.template,
            f(c.corr_growth_operating_margin, c.corr_growth_operating_margin_se),
            f(c.corr_growth_fcf_margin, c.corr_growth_fcf_margin_se)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The fade of fast growers
// ---------------------------------------------------------------------------------------------

pub const FADE_THRESHOLDS: [f64; 3] = [0.20, 0.30, 0.50];
pub const FADE_HORIZONS: [usize; 5] = [1, 2, 3, 5, 10];

#[derive(Debug, Clone, PartialEq)]
pub struct FadeHorizon {
    pub k: usize,
    pub n: usize,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FadeGroup {
    /// Size bucket label of the revenue of the start year, or `all`.
    pub size: String,
    pub threshold: f64,
    pub n_start: usize,
    pub start_mean: f64,
    pub start_median: f64,
    pub horizons: Vec<FadeHorizon>,
}

/// The growth `k` years after a year with growth above a threshold. A start year is a year with a growth
/// above the threshold, and the growth is organic (a jump with cash paid for businesses is not a start).
/// The later growth values are organic too. The size is the bucket of the revenue of the start year.
pub fn fade_groups(companies: &[CompanyPanel]) -> Vec<FadeGroup> {
    let mut out = Vec::new();
    let sizes: Vec<Option<usize>> = std::iter::once(None).chain((0..5).map(Some)).collect();
    for size in sizes {
        for thr in FADE_THRESHOLDS {
            let mut starts = Vec::new();
            let mut later: Vec<Vec<f64>> = vec![Vec::new(); FADE_HORIZONS.len()];
            for c in companies {
                for i in 1..c.rows.len() {
                    let r = &c.rows[i];
                    let Some(g) = r.growth_organic() else { continue };
                    if g <= thr || g >= GROWTH_LIMITS.1 || size.is_some_and(|s| bucket(r.revenue) != s) {
                        continue;
                    }
                    starts.push(g);
                    for (j, k) in FADE_HORIZONS.iter().enumerate() {
                        if i + k >= c.rows.len() || !c.rows[i + 1..=i + k].iter().all(|x| x.prev_ok) {
                            continue;
                        }
                        if let Some(v) = c.rows[i + k].growth_organic() {
                            later[j].push(v);
                        }
                    }
                }
            }
            if starts.is_empty() {
                continue;
            }
            let horizons = FADE_HORIZONS
                .iter()
                .zip(&later)
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| {
                    let s = sorted_copy(v);
                    FadeHorizon { k: *k, n: v.len(), p10: percentile(&s, 10.0), p50: percentile(&s, 50.0), p90: percentile(&s, 90.0) }
                })
                .collect();
            out.push(FadeGroup {
                size: size.map_or("all".to_string(), |s| BUCKET_LABELS[s].to_string()),
                threshold: thr,
                n_start: starts.len(),
                start_mean: starts.iter().sum::<f64>() / starts.len() as f64,
                start_median: percentile(&sorted_copy(&starts), 50.0),
                horizons,
            });
        }
    }
    out
}

fn render_fade(out: &mut String, companies: &[CompanyPanel], cells: &[ProcessPriors]) {
    let groups = fade_groups(companies);
    let _ = writeln!(out, "\nGROWTH FADE OF FAST GROWERS. Growth k years after a year with growth above a threshold and below 300%.");
    let _ = writeln!(out, "  A start is organic: a jump with cash paid for businesses, or a jump that reverses, is not a start. Size: revenue of the start year.");
    let _ = writeln!(out, "  Real: P10/P50/P90 of the growth k years later. AR1 pool: P10/P50/P90 of mean + phi^k (start median - mean) with the pooled estimates of");
    let _ = writeln!(out, "  the size bucket and the shock sd of the pooled model. AR1 FE+N: the median with phi of the within estimate with the Nickell correction.");
    let _ = writeln!(out, "  imp.phi = the one-year phi that the real median needs to reach the real median from the start median.");
    let find = |size: &str| cells.iter().find(|c| c.size == size && c.template == "all").and_then(|c| c.growth.as_ref());
    for g in &groups {
        let p = find(&g.size);
        let _ = writeln!(out, "\n  size {:<8} growth > {:>3.0}%   starts {:>5}  start growth: mean {} median {}", g.size, g.threshold * 100.0, g.n_start, pct(g.start_mean), pct(g.start_median));
        let _ = writeln!(out, "    {:>3} {:>6}   {:>7} {:>7} {:>7}   {:>7} {:>7} {:>7}   {:>8}  {:>7}", "k", "n", "P10", "P50", "P90", "AR1 P10", "AR1 P50", "AR1 P90", "AR1 FE+N", "imp.phi");
        for h in &g.horizons {
            let line = match p {
                Some(p) => {
                    let m = p.mean_pooled;
                    let phi = p.phi_pooled;
                    let pred = m + phi.powi(h.k as i32) * (g.start_median - m);
                    let var: f64 = (0..h.k).map(|j| phi.powi(2 * j as i32)).sum::<f64>() * p.sigma_pooled * p.sigma_pooled;
                    let (lo, hi) = (pred - 1.2816 * var.sqrt(), pred + 1.2816 * var.sqrt());
                    let fe = p.mean + p.phi.powi(h.k as i32) * (g.start_median - p.mean);
                    let ratio = (h.p50 - m) / (g.start_median - m);
                    let imp = if ratio > 0.0 { format!("{:7.3}", ratio.powf(1.0 / h.k as f64)) } else { "     <0".to_string() };
                    format!("{:>7} {:>7} {:>7}   {:>8}  {:>7}", pct(lo), pct(pred), pct(hi), pct(fe), imp)
                }
                None => "n/a".to_string(),
            };
            let _ = writeln!(out, "    {:>3} {:>6}   {:>7} {:>7} {:>7}   {line}", h.k, h.n, pct(h.p10), pct(h.p50), pct(h.p90));
        }
    }
    // The companies like a very large fast grower.
    struct Big {
        ticker: String,
        end: String,
        template: String,
        revenue: f64,
        before: f64,
        growth: f64,
        next: Option<f64>,
    }
    let mut big: Vec<Big> = Vec::new();
    for c in companies {
        for i in 1..c.rows.len() {
            let r = &c.rows[i];
            let Some(g) = r.growth_organic() else { continue };
            if g > 0.5 && r.revenue >= 10e9 {
                let next = c.rows.get(i + 1).filter(|n| n.prev_ok).and_then(|n| n.growth);
                big.push(Big { ticker: c.ticker.clone(), end: r.end.clone(), template: c.template.clone(), revenue: r.revenue, before: c.rows[i - 1].revenue, growth: g, next });
            }
        }
    }
    let count = |f: &dyn Fn(&Big) -> bool| big.iter().filter(|b| f(b)).count();
    let tickers = |f: &dyn Fn(&Big) -> bool| big.iter().filter(|b| f(b)).map(|b| b.ticker.clone()).collect::<std::collections::BTreeSet<_>>().len();
    let _ = writeln!(out, "\nVERY LARGE FAST GROWERS (organic growth above 50%, revenue of the year at least $10B)");
    let _ = writeln!(out, "  all templates: {} company-years, {} companies; growth > 100%: {}; revenue >= $30B: {}; revenue of the year before >= $10B: {}",
        big.len(), tickers(&|_| true), count(&|b| b.growth > 1.0), count(&|b| b.revenue >= 30e9), count(&|b| b.before >= 10e9));
    let _ = writeln!(out, "  standard template: {} company-years, {} companies; growth > 100%: {}; revenue >= $30B: {}; two years in sequence above 50%: {}",
        count(&|b| b.template == "standard"), tickers(&|b| b.template == "standard"), count(&|b| b.template == "standard" && b.growth > 1.0),
        count(&|b| b.template == "standard" && b.revenue >= 30e9), count(&|b| b.template == "standard" && b.next.is_some_and(|n| n > 0.5)));
    big.sort_by(|a, b| b.growth.partial_cmp(&a.growth).unwrap_or(std::cmp::Ordering::Equal));
    let _ = writeln!(out, "  the 40 largest growth values (template, revenue, revenue before, growth, growth of the next year):");
    for b in big.iter().take(40) {
        let next = b.next.map_or("      .".to_string(), pct);
        let _ = writeln!(out, "    {:<7} {} {:<8} {:7.1}B (before {:6.1}B)  growth {}  next {next}", b.ticker, b.end, b.template, b.revenue / 1e9, b.before / 1e9, pct(b.growth));
    }
}

/// The fade groups as table cells (written after the cells of the AR(1) table, as `[[fade]]`). `group_mean`
/// is the pooled mean growth of the size bucket, from the cell of the size and all templates.
pub fn fade_cells(companies: &[CompanyPanel], cells: &[ProcessPriors]) -> Vec<FadeCell> {
    let r6 = |x: f64| (x * 1e6).round() / 1e6;
    fade_groups(companies)
        .into_iter()
        .filter_map(|g| {
            let mean = cells.iter().find(|c| c.size == g.size && c.template == "all").and_then(|c| c.growth.as_ref())?.mean_pooled;
            Some(FadeCell {
                size: g.size,
                threshold: g.threshold,
                n_start: g.n_start,
                start_mean: r6(g.start_mean),
                start_median: r6(g.start_median),
                group_mean: r6(mean),
                step: g.horizons.iter().map(|h| FadeStep { k: h.k, n: h.n, p10: r6(h.p10), p50: r6(h.p50), p90: r6(h.p90) }).collect(),
            })
        })
        .collect()
}
