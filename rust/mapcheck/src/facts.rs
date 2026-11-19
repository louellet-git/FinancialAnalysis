//! SEC companyfacts data, and the selection of facts by period length.
//!
//! Rules:
//! - an annual value covers a full fiscal year (350 to 380 days);
//! - a quarterly value covers one quarter (80 to 100 days), or is the year-to-date
//!   value less the prior year-to-date value;
//! - a balance (instant) is selected by its end date;
//! - a duration fact with no start date has an unknown length and is accepted;
//! - when two filings report the same period, the latest filing wins.

use crate::dates;
use crate::registry::{Period, Tag};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

pub const ANNUAL_DAYS: (i64, i64) = (350, 380);
pub const QUARTER_DAYS: (i64, i64) = (80, 100);
pub const SAME_PERIOD_DAYS: i64 = 14;
pub const YTD_NOTE: &str = "year-to-date difference";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forms {
    Annual,
    Quarterly,
}

impl Forms {
    pub fn primary(self) -> &'static str {
        match self {
            Forms::Annual => "10-K",
            Forms::Quarterly => "10-Q",
        }
    }

    pub fn has(self, form: Option<&str>) -> bool {
        match (self, form) {
            (Forms::Annual, Some("10-K")) | (Forms::Annual, Some("10-K/A")) => true,
            (Forms::Quarterly, Some("10-Q")) | (Forms::Quarterly, Some("10-Q/A")) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Fact {
    pub start: Option<String>,
    pub end: String,
    pub val: Option<f64>,
    pub form: Option<String>,
    pub filed: Option<String>,
}

impl Fact {
    fn days(&self) -> Option<i64> {
        let start = dates::parse(self.start.as_deref()?)?;
        let end = dates::parse(&self.end)?;
        Some(end - start)
    }

    fn filed(&self) -> &str {
        self.filed.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Concept {
    #[serde(default)]
    pub units: HashMap<String, Vec<Fact>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CompanyFacts {
    #[serde(default)]
    pub cik: serde_json::Value,
    #[serde(rename = "entityName", default)]
    pub entity_name: Option<String>,
    #[serde(default)]
    pub facts: HashMap<String, HashMap<String, Concept>>,
}

impl CompanyFacts {
    /// All facts for one tag in one unit (an empty list if the filer has none).
    pub fn entries(&self, tag: &Tag, unit: &str) -> &[Fact] {
        self.facts
            .get(&tag.namespace)
            .and_then(|ns| ns.get(&tag.name))
            .and_then(|c| c.units.get(unit))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn has_us_gaap(&self) -> bool {
        self.facts.contains_key("us-gaap")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selected {
    pub value: f64,
    pub filed: String,
    pub note: &'static str,
}

fn within(days: Option<i64>, bounds: (i64, i64)) -> bool {
    match days {
        None => true,
        Some(d) => bounds.0 <= d && d <= bounds.1,
    }
}

/// The fact with the latest filing date (the first one wins when dates are equal).
fn latest<'a>(entries: impl Iterator<Item = &'a Fact>) -> Option<&'a Fact> {
    let mut best: Option<&Fact> = None;
    for e in entries {
        if e.val.is_none() {
            continue;
        }
        match best {
            None => best = Some(e),
            Some(b) if e.filed() > b.filed() => best = Some(e),
            _ => {}
        }
    }
    best
}

/// End dates of the periods that exist for these forms, oldest first.
pub fn period_ends(facts: &CompanyFacts, anchors: &[Tag], forms: Forms, n: Option<usize>) -> Vec<String> {
    let bounds = if forms == Forms::Annual { ANNUAL_DAYS } else { QUARTER_DAYS };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for anchor in anchors {
        for e in facts.entries(anchor, "USD") {
            if forms.has(e.form.as_deref()) && within(e.days(), bounds) {
                *counts.entry(e.end.clone()).or_default() += 1;
            }
        }
    }
    let ordered = merge_near_ends(&counts);
    match n {
        Some(n) if n > 0 && ordered.len() > n => ordered[ordered.len() - n..].to_vec(),
        _ => ordered,
    }
}

/// Two filings of one company can give the same period an end date that differs by
/// some days. End dates in a sequence with 14 days or less between neighbors are one
/// period: the date with the most anchor facts stays (if equal, the later date).
fn merge_near_ends(counts: &BTreeMap<String, usize>) -> Vec<String> {
    fn best(group: &[(&String, usize)]) -> String {
        // max by (count, date); the dates are in ascending sequence
        let mut winner = group[0];
        for &candidate in &group[1..] {
            if candidate.1 >= winner.1 {
                winner = candidate;
            }
        }
        winner.0.clone()
    }
    let mut kept = Vec::new();
    let mut group: Vec<(&String, usize)> = Vec::new();
    for (end, &count) in counts {
        if let Some((last, _)) = group.last() {
            let gap = match (dates::parse(end), dates::parse(last)) {
                (Some(a), Some(b)) => a - b,
                _ => i64::MAX,
            };
            if gap > SAME_PERIOD_DAYS {
                kept.push(best(&group));
                group.clear();
            }
        }
        group.push((end, count));
    }
    if !group.is_empty() {
        kept.push(best(&group));
    }
    kept
}

pub fn select_instant(entries: &[Fact], end: &str, forms: Forms) -> Option<Selected> {
    let best = latest(entries.iter().filter(|e| e.end == end && forms.has(e.form.as_deref())))?;
    Some(Selected { value: best.val?, filed: best.filed().to_string(), note: "" })
}

pub fn select_annual(entries: &[Fact], end: &str) -> Option<Selected> {
    let best = latest(entries.iter().filter(|e| {
        e.end == end && Forms::Annual.has(e.form.as_deref()) && within(e.days(), ANNUAL_DAYS)
    }))?;
    Some(Selected { value: best.val?, filed: best.filed().to_string(), note: "" })
}

pub fn select_quarter(entries: &[Fact], end: &str) -> Option<Selected> {
    let q = Forms::Quarterly;
    let here: Vec<&Fact> = entries.iter().filter(|e| e.end == end && q.has(e.form.as_deref())).collect();
    if let Some(best) = latest(here.iter().copied().filter(|e| within(e.days(), QUARTER_DAYS))) {
        return Some(Selected { value: best.val?, filed: best.filed().to_string(), note: "" });
    }
    let longest = here
        .iter()
        .filter(|e| e.val.is_some())
        .filter_map(|e| e.days())
        .filter(|&d| d > QUARTER_DAYS.1)
        .max()?;
    let ytd = latest(here.iter().copied().filter(|e| e.days() == Some(longest)))?;
    let end_day = dates::parse(end)?;
    let prior = latest(entries.iter().filter(|e| {
        e.start == ytd.start
            && q.has(e.form.as_deref())
            && dates::parse(&e.end)
                .map(|d| QUARTER_DAYS.0 <= end_day - d && end_day - d <= QUARTER_DAYS.1)
                .unwrap_or(false)
    }))?;
    let filed = if ytd.filed() >= prior.filed() { ytd.filed() } else { prior.filed() };
    Some(Selected { value: ytd.val? - prior.val?, filed: filed.to_string(), note: YTD_NOTE })
}

pub fn select(entries: &[Fact], end: &str, forms: Forms, period: Period) -> Option<Selected> {
    match (period, forms) {
        (Period::Instant, _) => select_instant(entries, end, forms),
        (Period::Duration, Forms::Annual) => select_annual(entries, end),
        (Period::Duration, Forms::Quarterly) => select_quarter(entries, end),
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;

    pub fn fact(start: Option<&str>, end: &str, val: f64, form: &str, filed: &str) -> Fact {
        Fact {
            start: start.map(str::to_string),
            end: end.to_string(),
            val: Some(val),
            form: Some(form.to_string()),
            filed: Some(filed.to_string()),
        }
    }

    /// Company facts from (tag name, facts) pairs in us-gaap, unit USD.
    pub fn company(concepts: Vec<(&str, Vec<Fact>)>) -> CompanyFacts {
        let mut gaap = HashMap::new();
        for (name, facts) in concepts {
            let mut units = HashMap::new();
            units.insert("USD".to_string(), facts);
            gaap.insert(name.to_string(), Concept { units });
        }
        let mut all = HashMap::new();
        all.insert("us-gaap".to_string(), gaap);
        CompanyFacts { cik: serde_json::Value::Null, entity_name: Some("ACME".into()), facts: all }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[test]
    fn annual_ignores_a_quarter_value_in_a_10k() {
        let entries = vec![
            fact(Some("2024-01-01"), "2024-12-31", 100.0, "10-K", "2025-02-01"),
            fact(Some("2024-10-01"), "2024-12-31", 27.0, "10-K", "2025-02-02"),
        ];
        assert_eq!(select_annual(&entries, "2024-12-31").unwrap().value, 100.0);
    }

    #[test]
    fn annual_is_none_when_only_a_quarter_exists() {
        let entries = vec![fact(Some("2024-10-01"), "2024-12-31", 27.0, "10-K", "2025-02-01")];
        assert!(select_annual(&entries, "2024-12-31").is_none());
    }

    #[test]
    fn annual_uses_the_latest_filing() {
        let entries = vec![
            fact(Some("2023-01-01"), "2023-12-31", 90.0, "10-K", "2024-02-01"),
            fact(Some("2023-01-01"), "2023-12-31", 88.0, "10-K", "2025-02-01"),
        ];
        assert_eq!(select_annual(&entries, "2023-12-31").unwrap().value, 88.0);
    }

    #[test]
    fn fact_with_no_start_is_accepted() {
        let entries = vec![fact(None, "2024-12-31", 100.0, "10-K", "2025-02-01")];
        assert_eq!(select_annual(&entries, "2024-12-31").unwrap().value, 100.0);
    }

    #[test]
    fn quarter_prefers_the_three_month_fact() {
        let entries = vec![
            fact(Some("2024-01-01"), "2024-06-30", 50.0, "10-Q", "2024-08-01"),
            fact(Some("2024-04-01"), "2024-06-30", 26.0, "10-Q", "2024-08-01"),
        ];
        let sel = select_quarter(&entries, "2024-06-30").unwrap();
        assert_eq!((sel.value, sel.note), (26.0, ""));
    }

    #[test]
    fn quarter_from_year_to_date_difference() {
        let entries = vec![
            fact(Some("2024-01-01"), "2024-03-31", 10.0, "10-Q", "2024-05-01"),
            fact(Some("2024-01-01"), "2024-06-30", 25.0, "10-Q", "2024-08-01"),
            fact(Some("2024-01-01"), "2024-09-30", 45.0, "10-Q", "2024-11-01"),
        ];
        let q2 = select_quarter(&entries, "2024-06-30").unwrap();
        let q3 = select_quarter(&entries, "2024-09-30").unwrap();
        assert_eq!((q2.value, q2.note), (15.0, YTD_NOTE));
        assert_eq!((q3.value, q3.note), (20.0, YTD_NOTE));
        assert_eq!(select_quarter(&entries, "2024-03-31").unwrap().value, 10.0);
    }

    #[test]
    fn quarter_is_none_when_the_prior_year_to_date_is_absent() {
        let entries = vec![fact(Some("2024-01-01"), "2024-09-30", 45.0, "10-Q", "2024-11-01")];
        assert!(select_quarter(&entries, "2024-09-30").is_none());
    }

    #[test]
    fn instant_selects_by_end_date_and_form() {
        let entries = vec![
            fact(None, "2024-12-31", 500.0, "10-K", "2025-02-01"),
            fact(None, "2024-12-31", 501.0, "10-Q", "2025-05-01"),
            fact(None, "2024-09-30", 480.0, "10-Q", "2024-11-01"),
        ];
        assert_eq!(select_instant(&entries, "2024-12-31", Forms::Annual).unwrap().value, 500.0);
        assert_eq!(select_instant(&entries, "2024-09-30", Forms::Quarterly).unwrap().value, 480.0);
        assert!(select_instant(&entries, "2024-06-30", Forms::Quarterly).is_none());
    }

    #[test]
    fn end_dates_that_are_near_each_other_are_one_period() {
        let facts = company(vec![
            (
                "Revenues",
                vec![
                    fact(Some("2015-11-02"), "2016-10-30", 100.0, "10-K", "2016-12-01"),
                    fact(Some("2015-11-01"), "2016-10-31", 100.0, "10-K", "2017-12-01"),
                    fact(Some("2016-10-31"), "2017-10-29", 110.0, "10-K", "2017-12-01"),
                ],
            ),
            ("NetIncomeLoss", vec![fact(Some("2015-11-02"), "2016-10-30", 9.0, "10-K", "2016-12-01")]),
        ]);
        let tag = |n: &str| Tag { full: format!("us-gaap/{n}"), namespace: "us-gaap".into(), name: n.into() };
        let anchors = vec![tag("Revenues"), tag("NetIncomeLoss")];
        assert_eq!(period_ends(&facts, &anchors, Forms::Annual, None), vec!["2016-10-30", "2017-10-29"]);
    }

    #[test]
    fn period_ends_annual_has_no_quarter_rows_and_keeps_the_latest_n() {
        let facts = company(vec![(
            "Revenues",
            vec![
                fact(Some("2022-01-01"), "2022-12-31", 80.0, "10-K", "2023-02-01"),
                fact(Some("2023-01-01"), "2023-12-31", 90.0, "10-K", "2024-02-01"),
                fact(Some("2024-01-01"), "2024-12-31", 100.0, "10-K", "2025-02-01"),
                fact(Some("2024-10-01"), "2024-12-31", 27.0, "10-K", "2025-02-01"),
                fact(Some("2024-07-01"), "2024-09-30", 26.0, "10-K", "2025-02-01"),
            ],
        )]);
        let anchors = vec![Tag { full: "us-gaap/Revenues".into(), namespace: "us-gaap".into(), name: "Revenues".into() }];
        assert_eq!(period_ends(&facts, &anchors, Forms::Annual, None), vec!["2022-12-31", "2023-12-31", "2024-12-31"]);
        assert_eq!(period_ends(&facts, &anchors, Forms::Annual, Some(2)), vec!["2023-12-31", "2024-12-31"]);
    }
}
