//! The annual panel of the company cache: one row for each company and fiscal year, with the rules
//! that make a clean growth series (ticket `T-20261119-dcf-process-calibration`, step 1).
//!
//! The panel is built from the company facts of the cache, with no network. The standard variables come
//! from `mapcheck` (the same mapping as `fa import`). The rules of one company (`clean_series`) work on
//! plain values (`RawYear`), so a test needs no cache.
//!
//! Rules (each one has a counter in `RuleCounts`):
//! 1. One row for one fiscal year. End dates that are 14 days or less apart are one year; the row of the
//!    latest filing stays. So a restated year has the value of the latest filing (the same rule as the
//!    mapping: `mapcheck::facts`).
//! 2. A row has a revenue above zero, or it is removed.
//! 3. Two rows are in sequence only if their end dates are 350 to 380 days apart (the same length as a
//!    fiscal year). A gap that is longer (a missing year) or shorter (a change of the fiscal year end) breaks
//!    the chain: the growth of that year is not defined. A 52-week or a 53-week year is in the 350 to 380
//!    range; a year with a gap above 366 days is counted (`long_years`) and not changed.
//! 4. A growth jump is flagged when the absolute growth is above 60% and the revenue of the year before is
//!    $1B or more. An acquisition or a spin-off is the likely cause. A jump is `acq_likely` when the company
//!    paid cash for businesses of at least 10% of the revenue change.
//! 5. Two jumps in sequence that undo each other (the revenue is back within 20% of the level before) are
//!    marked `glitch`: a wrong or recast value of one year is the likely cause.

use super::backtest::add_days;
use mapcheck::checks::filer_template;
use mapcheck::facts::{period_ends, CompanyFacts, Forms};
use mapcheck::mapper::{resolve_period_with, PeriodData};
use mapcheck::registry::Registry;
use mapcheck::run::read_facts;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const SAME_YEAR_DAYS: i64 = 14;
pub const YEAR_DAYS: (i64, i64) = (350, 380);
pub const LONG_YEAR_DAYS: i64 = 366;
pub const JUMP_GROWTH: f64 = 0.60;
pub const JUMP_MIN_REVENUE: f64 = 1e9;
/// Cash paid for businesses, as a share of the revenue change, that makes a jump `acq_likely`.
pub const ACQ_SHARE: f64 = 0.10;

/// Size buckets of revenue, in dollars.
pub const BUCKET_LIMITS: [f64; 4] = [1e9, 10e9, 30e9, 100e9];
pub const BUCKET_LABELS: [&str; 5] = ["<1B", "1-10B", "10-30B", "30-100B", ">100B"];

/// The size bucket of a revenue.
pub fn bucket(revenue: f64) -> usize {
    BUCKET_LIMITS.iter().take_while(|l| revenue >= **l).count()
}

/// One fiscal year before the rules.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawYear {
    pub end: String,
    pub filed: String,
    pub revenue: Option<f64>,
    pub operating_income: Option<f64>,
    pub operating_cash_flow: Option<f64>,
    /// Capital expenditure (the sign is not used).
    pub capex: Option<f64>,
    pub sbc: Option<f64>,
    /// Cash paid for businesses (the sign is not used).
    pub acquisitions: Option<f64>,
}

/// One fiscal year of the panel.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelRow {
    pub end: String,
    /// Calendar year of the middle of the fiscal year (a common label for all companies).
    pub year: i32,
    pub revenue: f64,
    pub operating_income: Option<f64>,
    pub operating_cash_flow: Option<f64>,
    pub capex: Option<f64>,
    pub fcf: Option<f64>,
    pub sbc: Option<f64>,
    pub acquisitions: Option<f64>,
    /// The row before is the year before (end dates 350 to 380 days apart).
    pub prev_ok: bool,
    /// The gap to the row before is above 366 days (a 53-week year).
    pub long_year: bool,
    /// Revenue growth of the year (`None` with no year before in the chain).
    pub growth: Option<f64>,
    /// The growth is a jump (see the rules).
    pub jump: bool,
    /// The jump has cash paid for businesses behind it.
    pub acq_likely: bool,
    /// The jump and the next (or the last) jump undo each other: the revenue is back within 20% of the
    /// level before. A wrong or recast value of one year is the likely cause.
    pub glitch: bool,
}

impl PanelRow {
    pub fn operating_margin(&self) -> Option<f64> {
        self.operating_income.map(|v| v / self.revenue)
    }

    pub fn fcf_margin(&self) -> Option<f64> {
        self.fcf.map(|v| v / self.revenue)
    }

    /// Growth with the jump rule applied: a jump is removed.
    pub fn growth_clean(&self) -> Option<f64> {
        if self.jump {
            None
        } else {
            self.growth
        }
    }

    /// Growth with the jumps that have an acquisition behind them, and the jumps that reverse, removed.
    pub fn growth_organic(&self) -> Option<f64> {
        if self.acq_likely || self.glitch {
            None
        } else {
            self.growth
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompanyPanel {
    pub ticker: String,
    pub name: String,
    /// `standard`, `bank`, `insurer`, `reit` or `fund` (`mapcheck::checks::filer_template` of the latest year).
    pub template: String,
    pub rows: Vec<PanelRow>,
}

/// How many items each rule removed or flagged.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RuleCounts {
    pub tickers_in_cache: usize,
    pub unreadable: usize,
    pub not_us_gaap: usize,
    pub no_annual_revenue: usize,
    pub companies: usize,
    pub years_in: usize,
    /// Rule 1.
    pub duplicate_years: usize,
    /// Rule 2.
    pub no_revenue: usize,
    pub rows_out: usize,
    /// Rule 3: pairs of rows in the panel, and the gaps.
    pub pairs: usize,
    pub gap_missing_year: usize,
    pub gap_short_year: usize,
    pub long_years: usize,
    pub growth_obs: usize,
    /// Rule 4.
    pub jumps: usize,
    pub jumps_acq_likely: usize,
    /// Rule 5: jumps that reverse in the next year.
    pub jumps_reversed: usize,
}

impl RuleCounts {
    pub fn add(&mut self, o: &RuleCounts) {
        self.tickers_in_cache += o.tickers_in_cache;
        self.unreadable += o.unreadable;
        self.not_us_gaap += o.not_us_gaap;
        self.no_annual_revenue += o.no_annual_revenue;
        self.companies += o.companies;
        self.years_in += o.years_in;
        self.duplicate_years += o.duplicate_years;
        self.no_revenue += o.no_revenue;
        self.rows_out += o.rows_out;
        self.pairs += o.pairs;
        self.gap_missing_year += o.gap_missing_year;
        self.gap_short_year += o.gap_short_year;
        self.long_years += o.long_years;
        self.growth_obs += o.growth_obs;
        self.jumps += o.jumps;
        self.jumps_acq_likely += o.jumps_acq_likely;
        self.jumps_reversed += o.jumps_reversed;
    }
}

fn day_gap(from: &str, to: &str) -> Option<i64> {
    let a = mapcheck::dates::parse(from)?;
    let b = mapcheck::dates::parse(to)?;
    Some(b - a)
}

fn year_of(date: &str) -> i32 {
    date.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0)
}

/// The rules 1 to 4 for the fiscal years of one company (in any sequence).
pub fn clean_series(mut raws: Vec<RawYear>, counts: &mut RuleCounts) -> Vec<PanelRow> {
    counts.years_in += raws.len();
    // Rule 1: sort by end date, and in a group of ends that are 14 days or less apart keep the latest filing.
    raws.sort_by(|a, b| (&a.end, &a.filed).cmp(&(&b.end, &b.filed)));
    let mut kept: Vec<RawYear> = Vec::new();
    for r in raws {
        match kept.last() {
            Some(last) if day_gap(&last.end, &r.end).is_some_and(|g| g <= SAME_YEAR_DAYS) => {
                counts.duplicate_years += 1;
                if r.filed >= last.filed {
                    *kept.last_mut().expect("not empty") = r;
                }
            }
            _ => kept.push(r),
        }
    }
    // Rule 2: a revenue above zero.
    let mut rows: Vec<PanelRow> = Vec::new();
    for r in kept {
        let Some(revenue) = r.revenue.filter(|v| *v > 0.0 && v.is_finite()) else {
            counts.no_revenue += 1;
            continue;
        };
        let capex = r.capex.map(f64::abs);
        rows.push(PanelRow {
            year: year_of(&add_days(&r.end, -182)),
            end: r.end,
            revenue,
            operating_income: r.operating_income,
            operating_cash_flow: r.operating_cash_flow,
            capex,
            fcf: match (r.operating_cash_flow, capex) {
                (Some(o), Some(c)) => Some(o - c),
                _ => None,
            },
            sbc: r.sbc,
            acquisitions: r.acquisitions.map(f64::abs),
            prev_ok: false,
            long_year: false,
            growth: None,
            jump: false,
            acq_likely: false,
            glitch: false,
        });
    }
    // Rules 3 and 4.
    for k in 1..rows.len() {
        counts.pairs += 1;
        let gap = day_gap(&rows[k - 1].end, &rows[k].end).unwrap_or(0);
        if gap > YEAR_DAYS.1 {
            counts.gap_missing_year += 1;
            continue;
        }
        if gap < YEAR_DAYS.0 {
            counts.gap_short_year += 1;
            continue;
        }
        let (prev, cur) = (rows[k - 1].revenue, rows[k].revenue);
        let growth = cur / prev - 1.0;
        let jump = growth.abs() > JUMP_GROWTH && prev >= JUMP_MIN_REVENUE;
        let acq = rows[k].acquisitions.unwrap_or(0.0);
        let acq_likely = jump && acq >= ACQ_SHARE * (cur - prev).abs();
        let row = &mut rows[k];
        row.prev_ok = true;
        row.long_year = gap > LONG_YEAR_DAYS;
        row.growth = Some(growth);
        row.jump = jump;
        row.acq_likely = acq_likely;
        counts.growth_obs += 1;
        counts.long_years += usize::from(row.long_year);
        counts.jumps += usize::from(jump);
        counts.jumps_acq_likely += usize::from(acq_likely);
    }
    // Rule 5: two jumps in sequence that undo each other.
    for k in 1..rows.len().saturating_sub(1) {
        if rows[k].jump && rows[k + 1].jump {
            let product = (1.0 + rows[k].growth.unwrap_or(0.0)) * (1.0 + rows[k + 1].growth.unwrap_or(0.0));
            if (0.8..=1.25).contains(&product) {
                for j in [k, k + 1] {
                    if !rows[j].glitch {
                        rows[j].glitch = true;
                        counts.jumps_reversed += 1;
                    }
                }
            }
        }
    }
    counts.rows_out += rows.len();
    rows
}

/// The fiscal years of the mapping of one company, as plain values.
fn raw_years(registry: &Registry, periods: &[PeriodData]) -> Vec<RawYear> {
    periods
        .iter()
        .map(|p| {
            let capex = p.get(registry, "capex_total").or_else(|| p.get(registry, "capex"));
            RawYear {
                end: p.end.clone(),
                filed: p.filed.clone().unwrap_or_default(),
                revenue: p.get(registry, "revenue"),
                operating_income: p.get(registry, "operating_income"),
                operating_cash_flow: p.get(registry, "operating_cash_flow"),
                capex,
                sbc: p.get(registry, "sbc_total"),
                acquisitions: p.get(registry, "acquisitions_net_of_cash"),
            }
        })
        .collect()
}

/// The panel of one company from its company facts (`None`: not a company for the panel).
pub fn company_panel(registry: &Registry, ticker: &str, facts: &CompanyFacts, counts: &mut RuleCounts) -> Option<CompanyPanel> {
    if !facts.has_us_gaap() {
        counts.not_us_gaap += 1;
        return None;
    }
    // All fiscal years that the facts give (no limit of periods).
    let ends = period_ends(facts, &registry.anchors, Forms::Annual, None);
    let periods: Vec<PeriodData> = ends.iter().map(|end| resolve_period_with(registry, facts, end, Forms::Annual, None)).collect();
    let template = periods.last().map(|p| filer_template(registry, p)).unwrap_or("standard").to_string();
    let rows = clean_series(raw_years(registry, &periods), counts);
    if rows.is_empty() {
        counts.no_annual_revenue += 1;
        return None;
    }
    counts.companies += 1;
    Some(CompanyPanel { ticker: ticker.to_string(), name: facts.entity_name.clone().unwrap_or_default(), template, rows })
}

/// The tickers that have a facts file in the cache, sorted.
pub fn cache_tickers(cache: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(cache).map_err(|e| format!("{}: {e}", cache.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('_') || name.starts_with('.') {
            continue;
        }
        if facts_file(cache, &name).is_some() {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

/// The facts file of a ticker; the compressed file first (it is smaller).
fn facts_file(cache: &Path, ticker: &str) -> Option<PathBuf> {
    ["company_facts.json.gz", "company_facts.json"].iter().map(|f| cache.join(ticker).join(f)).find(|p| p.is_file())
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub companies: Vec<CompanyPanel>,
    pub counts: RuleCounts,
    pub seconds: f64,
}

/// The panel of all tickers of the cache, in parallel. `only`: a list of tickers (all if `None`).
pub fn build_panel(cache: &Path, only: Option<&[String]>) -> Result<Panel, String> {
    let start = Instant::now();
    let registry = Registry::embedded()?;
    let tickers = match only {
        Some(list) => list.iter().map(|t| t.to_uppercase()).collect(),
        None => cache_tickers(cache)?,
    };
    let results: Vec<(Option<CompanyPanel>, RuleCounts)> = tickers
        .par_iter()
        .map(|t| {
            let mut counts = RuleCounts { tickers_in_cache: 1, ..Default::default() };
            let facts = match facts_file(cache, t).ok_or_else(|| "no file".to_string()).and_then(|p| read_facts(&p)) {
                Ok(f) => f,
                Err(_) => {
                    counts.unreadable += 1;
                    return (None, counts);
                }
            };
            let panel = company_panel(&registry, t, &facts, &mut counts);
            (panel, counts)
        })
        .collect();
    let mut counts = RuleCounts::default();
    let mut companies = Vec::new();
    for (panel, c) in results {
        counts.add(&c);
        companies.extend(panel);
    }
    companies.sort_by(|a, b| a.ticker.cmp(&b.ticker));
    Ok(Panel { companies, counts, seconds: start.elapsed().as_secs_f64() })
}

impl Panel {
    /// Company-years by size bucket of the revenue.
    pub fn rows_by_bucket(&self) -> [usize; 5] {
        let mut out = [0usize; 5];
        for c in &self.companies {
            for r in &c.rows {
                out[bucket(r.revenue)] += 1;
            }
        }
        out
    }

    /// Growth observations by size bucket of the revenue of the year before.
    pub fn growth_obs_by_bucket(&self) -> [usize; 5] {
        let mut out = [0usize; 5];
        for c in &self.companies {
            for k in 1..c.rows.len() {
                if c.rows[k].growth.is_some() {
                    out[bucket(c.rows[k - 1].revenue)] += 1;
                }
            }
        }
        out
    }

    pub fn companies_by_template(&self) -> Vec<(String, usize)> {
        let mut m = std::collections::BTreeMap::new();
        for c in &self.companies {
            *m.entry(c.template.clone()).or_insert(0usize) += 1;
        }
        m.into_iter().collect()
    }

    pub fn company_years(&self) -> usize {
        self.companies.iter().map(|c| c.rows.len()).sum()
    }

    /// The panel with only the rows of the fiscal years up to `year`; a company with no row left is dropped. It is the
    /// data of an out-of-sample table (`fa dcf calibrate --cut`, `fa dcf pit --cut`). The filter is by fiscal year, not
    /// by filing date: a later restatement of an old year stays (the values are today's facts).
    pub fn up_to_year(&self, year: i32) -> Panel {
        let companies = self
            .companies
            .iter()
            .map(|c| CompanyPanel { rows: c.rows.iter().filter(|r| r.year <= year).cloned().collect(), ..c.clone() })
            .filter(|c| !c.rows.is_empty())
            .collect::<Vec<CompanyPanel>>();
        // the counts of the kept rows; the counts of the rules (removed rows, jumps) stay those of the whole panel
        let counts = RuleCounts {
            companies: companies.len(),
            growth_obs: companies.iter().flat_map(|c| &c.rows).filter(|r| r.growth.is_some()).count(),
            ..self.counts
        };
        Panel { companies, counts, seconds: self.seconds }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(end: &str, filed: &str, revenue: f64) -> RawYear {
        RawYear { end: end.into(), filed: filed.into(), revenue: Some(revenue), ..Default::default() }
    }

    #[test]
    fn a_clean_series_has_growth_from_the_second_year() {
        let mut c = RuleCounts::default();
        let rows = clean_series(vec![raw("2021-12-31", "2022-02-01", 100.0), raw("2022-12-31", "2023-02-01", 110.0), raw("2023-12-31", "2024-02-01", 99.0)], &mut c);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].growth, None);
        assert!((rows[1].growth.unwrap() - 0.10).abs() < 1e-12);
        assert!((rows[2].growth.unwrap() + 0.10).abs() < 1e-12);
        assert_eq!((c.years_in, c.rows_out, c.growth_obs, c.jumps), (3, 3, 2, 0));
        assert_eq!(rows[1].year, 2022);
    }

    #[test]
    fn up_to_year_keeps_the_years_up_to_the_cut_drops_an_empty_company_and_counts_again() {
        let mut c = RuleCounts::default();
        let a = clean_series(vec![raw("2015-12-31", "2016-02-01", 100.0), raw("2016-12-31", "2017-02-01", 110.0), raw("2017-12-31", "2018-02-01", 121.0)], &mut c);
        let b = clean_series(vec![raw("2018-12-31", "2019-02-01", 50.0), raw("2019-12-31", "2020-02-01", 55.0)], &mut c);
        let company = |t: &str, rows: Vec<PanelRow>| CompanyPanel { ticker: t.into(), name: t.into(), template: "standard".into(), rows };
        let panel = Panel { companies: vec![company("A", a), company("B", b)], counts: c, seconds: 0.0 };
        let cut = panel.up_to_year(2016);
        assert_eq!(cut.companies.len(), 1); // B has no year up to 2016
        assert_eq!(cut.companies[0].rows.iter().map(|r| r.year).collect::<Vec<_>>(), vec![2015, 2016]);
        assert_eq!((cut.counts.companies, cut.counts.growth_obs), (1, 1));
        assert_eq!(cut.company_years(), 2);
        assert_eq!(cut.counts.years_in, panel.counts.years_in); // the counts of the rules stay those of the whole panel
        assert_eq!(panel.up_to_year(2030).company_years(), 5);
    }

    #[test]
    fn two_filings_of_one_year_give_one_row_with_the_latest_filing() {
        let mut c = RuleCounts::default();
        // a restated year: the end dates differ by 3 days, the later filing has the restated revenue
        let rows = clean_series(
            vec![raw("2022-12-31", "2023-02-01", 100.0), raw("2022-12-28", "2024-02-01", 90.0), raw("2023-12-31", "2024-02-01", 99.0)],
            &mut c,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(c.duplicate_years, 1);
        assert_eq!(rows[0].revenue, 90.0);
        assert!((rows[1].growth.unwrap() - 0.10).abs() < 1e-12);
    }

    #[test]
    fn a_53_week_year_is_kept_and_counted_and_a_short_year_breaks_the_chain() {
        let mut c = RuleCounts::default();
        let rows = clean_series(
            vec![
                raw("2019-09-28", "2019-11-01", 100.0),
                raw("2020-10-03", "2020-11-01", 102.0), // 371 days: 53 weeks
                raw("2021-10-02", "2021-11-01", 104.0), // 364 days
                raw("2022-03-31", "2022-05-01", 50.0),  // change of the fiscal year end: 180 days
                raw("2023-03-31", "2023-05-01", 55.0),
            ],
            &mut c,
        );
        assert_eq!(rows.len(), 5);
        assert!(rows[1].prev_ok && rows[1].long_year && rows[2].prev_ok && !rows[2].long_year);
        assert!(!rows[3].prev_ok && rows[3].growth.is_none());
        assert!(rows[4].prev_ok);
        assert_eq!((c.long_years, c.gap_short_year, c.gap_missing_year, c.growth_obs), (1, 1, 0, 3));
    }

    #[test]
    fn a_missing_year_breaks_the_chain_and_a_year_with_no_revenue_is_removed() {
        let mut c = RuleCounts::default();
        let mut missing = raw("2022-12-31", "2023-02-01", 0.0);
        missing.revenue = None;
        let rows = clean_series(vec![raw("2021-12-31", "2022-02-01", 100.0), missing, raw("2023-12-31", "2024-02-01", 120.0), raw("2024-12-31", "2025-02-01", 130.0)], &mut c);
        assert_eq!(rows.len(), 3);
        assert_eq!((c.no_revenue, c.gap_missing_year), (1, 1));
        assert!(!rows[1].prev_ok && rows[2].prev_ok);
    }

    #[test]
    fn a_big_jump_of_a_big_company_is_flagged_and_an_acquisition_is_recognized() {
        let mut c = RuleCounts::default();
        let mut y3 = raw("2023-12-31", "2024-02-01", 4e9);
        y3.acquisitions = Some(-1.2e9); // cash for businesses: 40% of the revenue change of 1.9B... above 10%
        let rows = clean_series(
            vec![
                raw("2020-12-31", "2021-02-01", 1.2e9),
                raw("2021-12-31", "2022-02-01", 2.1e9), // +75%, previous revenue above 1B: flagged, no cash for businesses
                raw("2022-12-31", "2023-02-01", 2.2e9),
                y3,                                     // +82%: flagged and acq_likely
                raw("2024-12-31", "2025-02-01", 4.2e9),
            ],
            &mut c,
        );
        assert!(rows[1].jump && !rows[1].acq_likely);
        assert!(!rows[2].jump);
        assert!(rows[3].jump && rows[3].acq_likely);
        assert_eq!((c.jumps, c.jumps_acq_likely), (2, 1));
        assert_eq!(rows[1].growth_clean(), None);
        assert!(rows[1].growth_organic().is_some());
        assert_eq!(rows[3].growth_organic(), None);
        // a small company with the same jump is not flagged
        let mut c2 = RuleCounts::default();
        let rows = clean_series(vec![raw("2021-12-31", "2022-02-01", 100e6), raw("2022-12-31", "2023-02-01", 200e6)], &mut c2);
        assert!(!rows[1].jump);
    }

    #[test]
    fn two_jumps_that_undo_each_other_are_a_glitch() {
        let mut c = RuleCounts::default();
        // a wrong value in one year: 5B, 1.8B, 5.2B
        let rows = clean_series(vec![raw("2020-12-31", "2021-02-01", 5e9), raw("2021-12-31", "2022-02-01", 1.8e9), raw("2022-12-31", "2023-02-01", 5.2e9), raw("2023-12-31", "2024-02-01", 5.5e9)], &mut c);
        assert!(rows[1].jump && rows[2].jump && rows[1].glitch && rows[2].glitch);
        assert_eq!(rows[2].growth_organic(), None);
        assert_eq!(c.jumps_reversed, 2);
        // a real jump that stays is not a glitch
        let mut c = RuleCounts::default();
        let rows = clean_series(vec![raw("2020-12-31", "2021-02-01", 5e9), raw("2021-12-31", "2022-02-01", 9e9), raw("2022-12-31", "2023-02-01", 9.5e9)], &mut c);
        assert!(rows[1].jump && !rows[1].glitch && rows[1].growth_organic().is_some());
    }

    #[test]
    fn buckets_follow_the_limits() {
        assert_eq!([bucket(0.5e9), bucket(1e9), bucket(9.9e9), bucket(10e9), bucket(35e9), bucket(100e9), bucket(300e9)], [0, 1, 1, 2, 3, 4, 4]);
    }
}
