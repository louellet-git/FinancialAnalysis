//! Rules that make the split table from the Alpaca rows and the SEC facts of the cache.
//!
//! Alpaca gives price factors of a spin-off or a merger as "splits" (HON, DELL, HLT, ...). Its history starts in 2016
//! and it misses real splits (BALL 2017, HWM 2016, EXE 2020). The SEC cover share count
//! (`dei:EntityCommonStockSharesOutstanding`) is the check and the second source:
//!
//! 1. A row with a ex date before the first SEC fact of the ticker is dropped (the ticker was used by another company).
//! 2. A row with `sec_check` = `match` stays.
//! 3. Another row stays if the cover count steps by its ratio (15%) within 120 days after the ex date. A flat cover
//!    count means a price factor of a spin-off or a merger: the row is dropped. If the cover count steps by a clear
//!    ratio that is not the factor and the factor is not a simple ratio (0.4873), the step ratio replaces the factor when
//!    it is a simple ratio to 1% (HLT 1:3). Where the cover count cannot decide (no cover fact, a new split, or a step
//!    that a merger can explain) the row stays if its factor is a simple ratio (2, 3, 1:10, 3:2, ...), and is dropped if
//!    it is not (a price factor is never round).
//! 4. A step of 1.8 times or more, or 0.55 times or less, between two cover facts, with no row of the table in the
//!    interval, is a split of its own when the weighted average share counts agree: a later filing gives the same
//!    period again, changed by the same ratio (a split changes the earlier periods; a merger does not). The row has
//!    `source` = "SEC cover-count step" and the cover date as an estimated ex date.
//! 5. `splits_corrections.csv` (in git) decides a case that no rule can decide: `keep`, `drop` or `set`.

use super::splits::*;
use std::collections::BTreeMap;

/// The data of one ticker from its company facts.
#[derive(Debug, Default, Clone)]
pub struct SecData {
    /// (cover date, shares) sorted by date.
    pub cover: Vec<(String, f64)>,
    /// The facts of the split ratio tag.
    pub tag: Vec<SecFact>,
    /// The first filing date of any fact.
    pub first_filed: Option<String>,
    /// (filing date, ratio): a later filing gives a period again with a share count that is 1.8 times or more, or
    /// 0.55 times or less, of the first filing (the earlier periods are changed by a split).
    pub recasts: Vec<(String, f64)>,
}

pub fn sec_data_from_json(v: &serde_json::Value) -> SecData {
    let mut by_date: BTreeMap<String, f64> = BTreeMap::new();
    for f in v["facts"]["dei"]["EntityCommonStockSharesOutstanding"]["units"]["shares"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        if let (Some(e), Some(x)) = (f["end"].as_str(), f["val"].as_f64()) {
            if x > 0.0 {
                let slot = by_date.entry(e.to_string()).or_insert(0.0);
                *slot = slot.max(x);
            }
        }
    }
    let mut first: Option<String> = None;
    for group in ["dei", "us-gaap"] {
        for tag in v["facts"][group].as_object().into_iter().flat_map(|m| m.values()) {
            for unit in tag["units"].as_object().into_iter().flat_map(|m| m.values()) {
                for f in unit.as_array().map(Vec::as_slice).unwrap_or(&[]) {
                    if let Some(d) = f["filed"].as_str() {
                        if first.as_deref().is_none_or(|x| d < x) {
                            first = Some(d.to_string());
                        }
                    }
                }
            }
        }
    }
    SecData { cover: by_date.into_iter().collect(), tag: sec_facts_from_json(v), first_filed: first, recasts: recasts_from_json(v) }
}

/// The recasts of the weighted average share counts (see `SecData::recasts`).
pub fn recasts_from_json(v: &serde_json::Value) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = Vec::new();
    for tag in ["WeightedAverageNumberOfSharesOutstandingBasic", "WeightedAverageNumberOfDilutedSharesOutstanding", "WeightedAverageNumberOfSharesOutstandingBasicAndDiluted"] {
        let mut groups: BTreeMap<(String, String), Vec<(String, f64)>> = BTreeMap::new();
        for f in v["facts"]["us-gaap"][tag]["units"]["shares"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
            if let (Some(s), Some(e), Some(x), Some(fl)) = (f["start"].as_str(), f["end"].as_str(), f["val"].as_f64(), f["filed"].as_str()) {
                if x > 0.0 {
                    groups.entry((s.to_string(), e.to_string())).or_default().push((fl.to_string(), x));
                }
            }
        }
        for g in groups.values_mut() {
            g.sort_by(|a, b| a.0.cmp(&b.0));
            let (_, first) = g[0].clone();
            for (filed, x) in g.iter().skip(1) {
                let r = x / first;
                if r >= 1.8 || r <= 0.55 {
                    out.push((filed.clone(), r));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The ratios that a company uses for a split: (new, old). 3 is 3:1 forward, (1, 3) is 1:3 reverse.
pub fn simple_ratios() -> Vec<(f64, f64)> {
    let ints = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 15.0, 16.0, 20.0, 25.0, 30.0, 40.0, 50.0, 100.0, 200.0];
    let mut v: Vec<(f64, f64)> = Vec::new();
    for n in ints {
        v.push((n, 1.0));
        v.push((1.0, n));
    }
    for (a, b) in [(3.0, 2.0), (5.0, 2.0), (7.0, 2.0), (4.0, 3.0), (5.0, 4.0)] {
        v.push((a, b));
        v.push((b, a));
    }
    v
}

/// The simple ratio nearest to `r`, when it is within `tol` (relative).
pub fn simple_ratio(r: f64, tol: f64) -> Option<(f64, f64)> {
    simple_ratios().into_iter().map(|(n, o)| ((n / o / r - 1.0).abs(), (n, o))).filter(|(e, _)| *e <= tol).min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, x)| x)
}

pub fn is_simple(factor: f64) -> bool {
    simple_ratio(factor, 1e-4).is_some()
}

#[derive(Debug, PartialEq)]
pub enum Evidence {
    /// The cover count steps by the factor (15%).
    Confirms,
    /// A clear step (1.8 times or more, 0.55 times or less) that is not the factor: the ratio.
    Step(f64),
    Flat,
    /// No cover fact after the ex date yet.
    TooNew,
    NoCover,
}

/// The cover count steps: the last fact before the ex date (at most 400 days before) against the facts up to 120 days after.
pub fn evidence(cover: &[(String, f64)], ex: &str, factor: f64) -> Evidence {
    let Some(exd) = days(ex) else { return Evidence::NoCover };
    let dd = |e: &str| days(e).unwrap_or(i32::MIN);
    let Some((_, before)) = cover.iter().rfind(|(e, _)| dd(e) < exd && exd - dd(e) <= 400) else { return Evidence::NoCover };
    let afters: Vec<f64> = cover.iter().filter(|(e, _)| dd(e) > exd && dd(e) <= exd + SEC_WINDOW_DAYS).map(|(_, v)| *v).collect();
    if afters.is_empty() {
        return if cover.iter().all(|(e, _)| dd(e) <= exd) { Evidence::TooNew } else { Evidence::NoCover };
    }
    // A factor near 1 (0.87 to 1.15) is not told from a flat count.
    let clear = factor >= 1.15 || factor <= 0.87;
    // The count must move: a count within 10% of the old one is flat, whatever the factor.
    if clear && afters.iter().any(|a| (a / before / factor - 1.0).abs() <= 0.15 && !(0.9..=1.1).contains(&(a / before))) {
        return Evidence::Confirms;
    }
    let r0 = afters[0] / before;
    if r0 >= 1.8 || r0 <= 0.55 {
        Evidence::Step(r0)
    } else {
        Evidence::Flat
    }
}

/// Does a recast of the share counts agree with a cover step `r` between `d1` and `d2` (days)? The filing is after
/// the earlier cover date and at most 120 days after the later one; the ratio agrees to 5%.
pub fn recast_confirms(recasts: &[(String, f64)], d1: i32, d2: i32, r: f64) -> bool {
    recasts.iter().any(|(filed, x)| days(filed).is_some_and(|d| d > d1 && d <= d2 + 120) && (x / r - 1.0).abs() <= 0.05)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub ticker: String,
    pub ex_date: String,
    pub factor: f64,
    /// `dropped`, `kept` (not confirmed), `added`, `corrected` or `file`.
    pub what: &'static str,
    pub why: String,
}

#[derive(Debug, Clone)]
pub struct Correction {
    pub ticker: String,
    pub ex_date: String,
    pub action: String,
    pub old_rate: f64,
    pub new_rate: f64,
}

pub fn parse_corrections(text: &str) -> Result<Vec<Correction>, String> {
    let mut out = Vec::new();
    for (i, l) in text.lines().enumerate().skip(1).filter(|(_, l)| !l.trim().is_empty()) {
        let f: Vec<&str> = l.split(',').collect();
        let num = |k: usize| f.get(k).and_then(|x| x.trim().parse::<f64>().ok());
        let action = f.get(2).map(|a| a.trim()).unwrap_or("");
        if f.len() < 6 || !matches!(action, "keep" | "drop" | "set") || days(f[1].trim()).is_none() {
            return Err(format!("splits_corrections.csv line {}: bad row", i + 1));
        }
        let (old_rate, new_rate) = (num(3).unwrap_or(1.0), num(4).unwrap_or(1.0));
        if !(old_rate > 0.0 && new_rate > 0.0) {
            return Err(format!("splits_corrections.csv line {}: bad rate", i + 1));
        }
        out.push(Correction { ticker: f[0].trim().to_uppercase(), ex_date: f[1].trim().into(), action: action.into(), old_rate, new_rate });
    }
    Ok(out)
}

pub fn builtin_corrections() -> Vec<Correction> {
    parse_corrections(include_str!("splits_corrections.csv")).expect("splits_corrections.csv is valid")
}

fn status_of(ex: &str, price_last: Option<&str>) -> &'static str {
    match price_last {
        Some(l) if ex > l => "pending",
        _ => "applied",
    }
}

fn make_split(ticker: &str, ex: &str, new: f64, old: f64, tag: &[SecFact]) -> Split {
    let factor = new / old;
    Split { ticker: ticker.into(), ex_date: ex.into(), old_rate: old, new_rate: new, factor, kind: if factor > 1.0 { "forward" } else { "reverse" }.into(), sec_check: sec_check(ex, factor, tag).into() }
}

/// The rows of one ticker. `alpaca` are its Alpaca rows. `start` is the first date of the table.
pub fn rules_for_ticker(ticker: &str, alpaca: &[Split], sec: &SecData, price_last: Option<&str>, start: &str, corrections: &[Correction]) -> (Vec<SplitRow>, Vec<Event>) {
    let mut rows: Vec<SplitRow> = Vec::new();
    let mut events = Vec::new();
    // Rows dropped because the cover count did not move after the ex date: the step can be before it (FFIN 2019).
    let mut flat: Vec<(Split, Event)> = Vec::new();
    let ev = |s: &Split, what: &'static str, why: String| Event { ticker: ticker.into(), ex_date: s.ex_date.clone(), factor: s.factor, what, why };
    for a in alpaca {
        let mut s = a.clone();
        s.sec_check = sec_check(&s.ex_date, s.factor, &sec.tag).into();
        let status = status_of(&s.ex_date, price_last);
        if let Some(c) = corrections.iter().find(|c| c.ticker == ticker && c.ex_date == s.ex_date) {
            match c.action.as_str() {
                "drop" => events.push(ev(&s, "dropped", "correction file: drop".into())),
                "keep" => {
                    rows.push(SplitRow::new(s.clone(), SRC_FILE, status, false));
                    events.push(ev(&s, "file", "correction file: keep".into()));
                }
                _ => {
                    let n = make_split(ticker, &s.ex_date, c.new_rate, c.old_rate, &sec.tag);
                    events.push(ev(&n, "file", format!("correction file: set {}:{} (Alpaca factor {})", c.old_rate, c.new_rate, s.factor)));
                    rows.push(SplitRow::new(n, SRC_FILE, status, false));
                }
            }
            continue;
        }
        if let Some(f) = sec.first_filed.as_deref().filter(|f| s.ex_date.as_str() < *f) {
            events.push(ev(&s, "dropped", format!("F5: ex date is before the first SEC fact of the ticker ({f}): ticker reuse")));
            continue;
        }
        if s.sec_check == "match" {
            rows.push(SplitRow::new(s, SRC_MATCH, status, false));
            continue;
        }
        let unconfirmed = |s: Split| SplitRow::new(s.clone(), if s.sec_check == "no_sec_fact" { SRC_NO_FACT } else { SRC_UNCONFIRMED }, status, false);
        match evidence(&sec.cover, &s.ex_date, s.factor) {
            Evidence::Confirms => rows.push(SplitRow::new(s, SRC_COVER, status, false)),
            Evidence::Flat => flat.push((s.clone(), ev(&s, "dropped", "F1: the cover count does not move: price factor of a spin-off or a merger".into()))),
            Evidence::Step(r) => match simple_ratio(r, 0.01) {
                Some((n, o)) if !is_simple(s.factor) => {
                    let c = make_split(ticker, &s.ex_date, n, o, &sec.tag);
                    events.push(ev(&c, "corrected", format!("cover count steps {r:.4} times; Alpaca factor {} replaced by {n}:{o}", s.factor)));
                    rows.push(SplitRow::new(c, SRC_CORRECTED, status, false));
                }
                _ if is_simple(s.factor) => {
                    events.push(ev(&s, "kept", format!("simple factor; the cover count steps {r:.4} times (a merger or an offering can explain it): not confirmed")));
                    rows.push(unconfirmed(s));
                }
                _ => events.push(ev(&s, "dropped", format!("cover count steps {r:.4} times: not the factor, and the factor is not a simple ratio"))),
            },
            Evidence::TooNew | Evidence::NoCover if is_simple(s.factor) => {
                events.push(ev(&s, "kept", "simple factor; no cover count to check it: not confirmed".into()));
                rows.push(unconfirmed(s));
            }
            Evidence::TooNew | Evidence::NoCover => events.push(ev(&s, "dropped", "F1: the factor is not a simple ratio and no cover count confirms it".into())),
        }
    }
    // The second source: steps of the cover count that no row explains.
    let c = &sec.cover;
    for i in 1..c.len() {
        let ((e1, v1), (e2, v2)) = (&c[i - 1], &c[i]);
        let (Some(d1), Some(d2)) = (days(e1), days(e2)) else { continue };
        if d2 - d1 > 250 || e2.as_str() <= start {
            continue;
        }
        let r = v2 / v1;
        if r < 1.8 && r > 0.55 {
            continue;
        }
        let Some((n, o)) = simple_ratio(r, 0.03) else { continue };
        // A step that goes back at the next fact, or that comes back from the step before, is a data fault.
        if c.get(i + 1).is_some_and(|(_, v3)| ((v3 / v2) * r - 1.0).abs() <= 0.15) || (i >= 2 && ((v1 / c[i - 2].1) * r - 1.0).abs() <= 0.15) {
            continue;
        }
        if !recast_confirms(&sec.recasts, d1, d2, r) {
            continue;
        }
        let explained = rows.iter().any(|x| days(&x.split.ex_date).is_some_and(|d| d > d1 && d <= d2 + 10));
        if explained {
            continue;
        }
        // The cover count was already at the new level when Alpaca's ex date came: use the Alpaca date.
        if let Some(k) = flat.iter().position(|(a, _)| days(&a.ex_date).is_some_and(|d| d > d2 && d <= d2 + 120) && (a.factor / (n / o) - 1.0).abs() <= 0.05) {
            let (a, _) = flat.remove(k);
            let split = make_split(ticker, &a.ex_date, n, o, &sec.tag);
            events.push(Event { ticker: ticker.into(), ex_date: a.ex_date.clone(), factor: split.factor, what: "corrected", why: format!("cover count {v1} on {e1} to {v2} on {e2} ({r:.4} times) is before the Alpaca ex date: Alpaca date, simple factor") });
            rows.push(SplitRow::new(split, SRC_CORRECTED, status_of(&a.ex_date, price_last), false));
            continue;
        }
        let split = make_split(ticker, e2, n, o, &sec.tag);
        events.push(Event { ticker: ticker.into(), ex_date: e2.clone(), factor: split.factor, what: "added", why: format!("cover count {v1} on {e1} to {v2} on {e2} ({r:.4} times)") });
        rows.push(SplitRow::new(split, SRC_STEP, status_of(e2, price_last), true));
    }
    events.extend(flat.into_iter().map(|(_, e)| e));
    (rows, events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(d: &str, new: f64, old: f64) -> Split {
        Split { ticker: "T".into(), ex_date: d.into(), old_rate: old, new_rate: new, factor: new / old, kind: "forward".into(), sec_check: String::new() }
    }
    fn cover(v: &[(&str, f64)]) -> Vec<(String, f64)> {
        v.iter().map(|(d, x)| (d.to_string(), *x)).collect()
    }
    fn data(c: &[(&str, f64)]) -> SecData {
        SecData { cover: cover(c), tag: vec![], first_filed: Some("2010-01-01".into()), recasts: vec![] }
    }
    fn run(a: &[Split], d: &SecData) -> (Vec<SplitRow>, Vec<Event>) {
        rules_for_ticker("T", a, d, Some("2026-11-19"), "2009-01-01", &[])
    }

    #[test]
    fn ratios() {
        assert_eq!(simple_ratio(0.3331, 0.05), Some((1.0, 3.0)));
        assert_eq!(simple_ratio(1.99, 0.03), Some((2.0, 1.0)));
        assert_eq!(simple_ratio(0.4725, 0.05), None);
        assert!(is_simple(25.0) && is_simple(0.1) && is_simple(1.25) && is_simple(1.0 / 15.0) && !is_simple(0.48732) && !is_simple(1.0306));
    }

    #[test]
    fn cover_evidence() {
        let c = cover(&[("2020-01-31", 1000.0), ("2020-04-30", 2010.0)]);
        assert_eq!(evidence(&c, "2020-03-01", 2.0), Evidence::Confirms);
        assert!(matches!(evidence(&c, "2020-03-01", 0.48), Evidence::Step(_)));
        let flat = cover(&[("2020-01-31", 1000.0), ("2020-04-30", 1005.0)]);
        assert_eq!(evidence(&flat, "2020-03-01", 0.48), Evidence::Flat);
        assert_eq!(evidence(&flat, "2020-03-01", 1.03), Evidence::Flat, "a factor near 1 is not confirmed");
        assert_eq!(evidence(&flat, "2020-06-01", 2.0), Evidence::TooNew);
        assert_eq!(evidence(&flat, "2019-01-01", 2.0), Evidence::NoCover);
        assert_eq!(evidence(&[], "2020-03-01", 2.0), Evidence::NoCover);
    }

    #[test]
    fn f1_price_factor_is_dropped_and_a_confirmed_row_stays() {
        let d = data(&[("2020-01-31", 1000.0), ("2020-04-30", 1002.0), ("2021-01-31", 2004.0), ("2021-04-30", 2000.0)]);
        let (rows, ev) = run(&[sp("2020-03-01", 0.48, 1.0), sp("2020-12-01", 2.0, 1.0)], &d);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].split.ex_date, "2020-12-01");
        assert_eq!(rows[0].source, SRC_COVER);
        assert_eq!(ev.len(), 1);
        assert!(ev[0].what == "dropped" && ev[0].why.starts_with("F1"));
    }

    #[test]
    fn f3_the_step_replaces_a_price_factor() {
        let d = data(&[("2016-10-21", 989.8), ("2017-02-08", 329.7)]);
        let (rows, ev) = run(&[sp("2017-01-04", 0.48732, 1.0)], &d);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].split.old_rate, rows[0].split.new_rate, rows[0].split.kind.as_str()), (3.0, 1.0, "reverse"));
        assert_eq!(rows[0].source, SRC_CORRECTED);
        assert_eq!(ev[0].what, "corrected");
    }

    #[test]
    fn f4_a_step_with_no_row_is_added() {
        let mut d = data(&[("2017-04-21", 170.0), ("2017-07-24", 345.0), ("2017-10-24", 344.0)]);
        // a merger gives the same step, but no later filing changes the earlier periods
        assert!(run(&[], &d).0.is_empty());
        d.recasts = vec![("2017-08-01".into(), 2.0)];
        let (rows, ev) = run(&[], &d);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].split.ex_date.as_str(), rows[0].split.factor, rows[0].source.as_str(), rows[0].ex_date_estimated), ("2017-07-24", 2.0, SRC_STEP, true));
        assert_eq!(ev[0].what, "added");
        // a row of the table in the interval explains the step
        let (rows, _) = run(&[sp("2017-06-30", 2.0, 1.0)], &d);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, SRC_COVER);
        // a step that goes back is a data fault
        let mut g = data(&[("2017-04-21", 170.0), ("2017-07-24", 345.0), ("2017-10-24", 171.0)]);
        g.recasts = d.recasts.clone();
        assert!(run(&[], &g).0.is_empty());
        // a step that is not a simple ratio is not a split
        let mut m = data(&[("2017-04-21", 170.0), ("2017-07-24", 310.0)]);
        m.recasts = d.recasts.clone();
        assert!(run(&[], &m).0.is_empty());
        // a gap of more than 250 days is not tested
        let mut w = data(&[("2016-04-21", 170.0), ("2017-07-24", 345.0)]);
        w.recasts = d.recasts.clone();
        assert!(run(&[], &w).0.is_empty());
    }

    #[test]
    fn simple_factor_stays_when_the_cover_count_cannot_decide() {
        // no cover count at all (two share classes): a round factor stays, a price factor goes
        let none = data(&[]);
        let (rows, _) = run(&[sp("2018-03-01", 5.0, 4.0), sp("2018-04-01", 1.973, 1.0)], &none);
        assert_eq!((rows.len(), rows[0].source.as_str()), (1, SRC_NO_FACT));
        // a step that a merger can explain, and a round Alpaca factor: the row stays, not confirmed
        let step = data(&[("2018-07-31", 100.0), ("2018-11-26", 410.0)]);
        let (rows, ev) = run(&[sp("2018-10-05", 3.0, 1.0)], &step);
        assert_eq!((rows[0].split.factor, ev[0].what), (3.0, "kept"));
        // 1.5% from a simple ratio is no correction of a price factor
        let (rows, _) = run(&[sp("2018-10-05", 0.8824, 1.0)], &data(&[("2018-07-31", 100.0), ("2018-11-26", 203.0)]));
        assert!(rows.is_empty());
    }

    #[test]
    fn a_step_before_the_alpaca_date_keeps_the_alpaca_date() {
        let mut d = data(&[("2019-01-31", 100.0), ("2019-04-30", 201.0), ("2019-07-31", 202.0)]);
        d.recasts = vec![("2019-05-10".into(), 2.0)];
        let (rows, ev) = run(&[sp("2019-06-04", 2.0, 1.0)], &d);
        assert_eq!((rows.len(), rows[0].split.ex_date.as_str(), rows[0].ex_date_estimated, rows[0].source.as_str()), (1, "2019-06-04", false, SRC_CORRECTED));
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].what, "corrected");
    }

    #[test]
    fn a_flat_count_is_not_a_confirmation() {
        // DXC: factor 1.156, the count moves 1.3%
        let c = cover(&[("2018-05-11", 284.8), ("2018-07-31", 281.2)]);
        assert_eq!(evidence(&c, "2018-06-01", 1.156), Evidence::Flat);
    }

    #[test]
    fn recasts_of_the_share_counts() {
        let v = serde_json::json!({"facts": {"us-gaap": {"WeightedAverageNumberOfSharesOutstandingBasic": {"units": {"shares": [
            {"start": "2017-01-01", "end": "2017-03-31", "val": 100, "filed": "2017-05-01"},
            {"start": "2017-01-01", "end": "2017-03-31", "val": 200, "filed": "2018-05-01"},
            {"start": "2017-04-01", "end": "2017-06-30", "val": 100, "filed": "2017-08-01"},
            {"start": "2017-04-01", "end": "2017-06-30", "val": 101, "filed": "2018-08-01"}]}}}}});
        assert_eq!(recasts_from_json(&v), vec![("2018-05-01".to_string(), 2.0)]);
        assert!(recast_confirms(&[("2018-05-01".into(), 2.0)], days("2018-01-31").unwrap(), days("2018-04-30").unwrap(), 2.05));
        assert!(!recast_confirms(&[("2018-05-01".into(), 2.0)], days("2018-01-31").unwrap(), days("2018-04-30").unwrap(), 3.0));
        assert!(!recast_confirms(&[("2017-05-01".into(), 2.0)], days("2018-01-31").unwrap(), days("2018-04-30").unwrap(), 2.0), "a filing before the step");
    }

    #[test]
    fn f5_ticker_reuse_is_dropped() {
        let mut d = data(&[("2016-12-31", 100.0)]);
        d.first_filed = Some("2016-11-25".into());
        let (rows, ev) = run(&[sp("2016-10-06", 2.0, 1.0)], &d);
        assert!(rows.is_empty());
        assert!(ev[0].why.starts_with("F5"));
    }

    #[test]
    fn f6_status_and_a_new_split() {
        let d = data(&[("2026-07-31", 80.0)]);
        let (rows, _) = run(&[sp("2026-11-20", 0.125, 1.0)], &d);
        assert_eq!((rows[0].status.as_str(), rows[0].source.as_str()), ("pending", SRC_NO_FACT));
        // a new split with a factor that is not a simple ratio is dropped
        assert!(run(&[sp("2026-11-20", 0.4873, 1.0)], &d).0.is_empty());
        // a match row stays with no cover fact
        let mut m = data(&[]);
        m.tag = vec![("2021-07-19".into(), 4.0)];
        let (rows, _) = run(&[sp("2021-07-20", 4.0, 1.0)], &m);
        assert_eq!((rows[0].status.as_str(), rows[0].split.sec_check.as_str(), rows[0].source.as_str()), ("applied", "match", SRC_MATCH));
    }

    #[test]
    fn correction_file() {
        let c = parse_corrections("ticker,ex_date,action,old_rate,new_rate,reason\nT,2020-03-01,set,3,1,test\nT,2020-04-01,drop,1,1,test\n").unwrap();
        let d = data(&[]);
        let (rows, ev) = rules_for_ticker("T", &[sp("2020-03-01", 0.48, 1.0), sp("2020-04-01", 2.0, 1.0)], &d, None, "2009-01-01", &c);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].split.factor, rows[0].source.as_str()), (1.0 / 3.0, SRC_FILE));
        assert_eq!(ev.len(), 2);
        assert!(parse_corrections("h\nT,bad,keep,1,1,x\n").is_err());
        assert!(builtin_corrections().len() < 50);
    }

    #[test]
    fn sec_data_from_a_text() {
        let v = serde_json::json!({"facts": {"dei": {"EntityCommonStockSharesOutstanding": {"units": {"shares": [
            {"end": "2020-01-31", "val": 5, "filed": "2020-02-10"}, {"end": "2020-01-31", "val": 7, "filed": "2020-03-10"}, {"end": "2019-10-31", "val": 4, "filed": "2019-11-10"}]}}},
            "us-gaap": {"X": {"units": {"USD": [{"end": "2018-12-31", "val": 1, "filed": "2019-02-01"}]}}}}});
        let s = sec_data_from_json(&v);
        assert_eq!(s.cover, vec![("2019-10-31".to_string(), 4.0), ("2020-01-31".to_string(), 7.0)]);
        assert_eq!(s.first_filed.as_deref(), Some("2019-02-01"));
    }
}
