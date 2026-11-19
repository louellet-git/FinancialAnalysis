//! Rate series, intervals, and resampling to a sampling frequency.

use mapcheck::dates;
use serde::Serialize;

pub type Result<T> = std::result::Result<T, String>;

/// The fewest rate observations (after resampling) that a calibration accepts.
pub const MIN_OBS: usize = 30;

/// A series of rates: day numbers (days from 1970-01-01, increasing) and decimal values.
#[derive(Debug, Clone)]
pub struct Series {
    pub name: String,
    pub days: Vec<i64>,
    pub values: Vec<f64>,
}

impl Series {
    pub fn new(name: &str, days: Vec<i64>, values: Vec<f64>) -> Result<Series> {
        if days.len() != values.len() {
            return Err("series: days and values have different lengths".into());
        }
        if days.windows(2).any(|w| w[1] <= w[0]) {
            return Err("series: the days are not increasing".into());
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err("series: a value is not finite".into());
        }
        Ok(Series { name: name.to_string(), days, values })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
}

impl Freq {
    pub fn parse(text: &str) -> Result<Freq> {
        match text {
            "daily" => Ok(Freq::Daily),
            "weekly" => Ok(Freq::Weekly),
            "monthly" => Ok(Freq::Monthly),
            other => Err(format!("bad frequency '{other}': use daily, weekly or monthly")),
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Freq::Daily => "daily",
            Freq::Weekly => "weekly",
            Freq::Monthly => "monthly",
        }
    }
}

/// The rates of one interval at one frequency. The step `dt` (years) is the mean calendar
/// spacing: (last day - first day) / 365.25 / (n - 1).
#[derive(Debug, Clone)]
pub struct Sample {
    pub series: String,
    pub freq: Freq,
    pub days: Vec<i64>,
    pub r: Vec<f64>,
    pub dt: f64,
    pub warnings: Vec<String>,
}

pub fn civil(day: i64) -> (i64, i64, i64) {
    let t = dates::format(day);
    (t[0..4].parse().unwrap_or(0), t[5..7].parse().unwrap_or(1), t[8..10].parse().unwrap_or(1))
}

/// Add whole months. The day of the month is cut to the end of a shorter month.
pub fn add_months(day: i64, months: i64) -> i64 {
    let (y, m, d) = civil(day);
    let total = y * 12 + (m - 1) + months;
    let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    let mut nd = d;
    while nd > 28 && dates::format(dates::days_from_civil(ny, nm, nd))[5..7].parse::<i64>().unwrap_or(nm) != nm {
        nd -= 1;
    }
    dates::days_from_civil(ny, nm, nd)
}

/// A length of time for a window or a step: "5y", "3m", "2w", "10d".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub months: i64,
    pub days: i64,
}

impl Span {
    pub fn parse(text: &str) -> Result<Span> {
        let bad = || format!("bad time span '{text}': use a number and y, m, w or d, for example 5y, 3m, 2w, 10d");
        let (num, unit) = text.split_at(text.len().saturating_sub(1));
        let n: i64 = num.parse().map_err(|_| bad())?;
        if n <= 0 {
            return Err(bad());
        }
        match unit {
            "y" => Ok(Span { months: 12 * n, days: 0 }),
            "m" => Ok(Span { months: n, days: 0 }),
            "w" => Ok(Span { months: 0, days: 7 * n }),
            "d" => Ok(Span { months: 0, days: n }),
            _ => Err(bad()),
        }
    }
    pub fn add_to(&self, day: i64) -> i64 {
        add_months(day, self.months) + self.days
    }
}

fn period_key(day: i64, freq: Freq) -> i64 {
    match freq {
        Freq::Daily => day,
        // 1970-01-01 was a Thursday, so (day + 3) / 7 starts a new week on Monday.
        Freq::Weekly => (day + 3).div_euclid(7),
        Freq::Monthly => {
            let (y, m, _) = civil(day);
            y * 12 + m - 1
        }
    }
}

/// The observations of `series` from `start` to `end` (both days included), at the frequency
/// `freq`. For weekly and monthly data the sample takes the last observation of each period.
/// Gives an error if fewer than `MIN_OBS` observations remain.
pub fn sample(series: &Series, start: Option<i64>, end: Option<i64>, freq: Freq) -> Result<Sample> {
    let (lo, hi) = (start.unwrap_or(i64::MIN), end.unwrap_or(i64::MAX));
    let mut days: Vec<i64> = Vec::new();
    let mut r: Vec<f64> = Vec::new();
    for (i, &d) in series.days.iter().enumerate() {
        if d < lo || d > hi {
            continue;
        }
        if let Some(&last) = days.last() {
            if period_key(last, freq) == period_key(d, freq) {
                *days.last_mut().expect("not empty") = d;
                *r.last_mut().expect("not empty") = series.values[i];
                continue;
            }
        }
        days.push(d);
        r.push(series.values[i]);
    }
    if r.len() < MIN_OBS {
        return Err(format!(
            "{} has {} {} observations in the interval; a calibration needs at least {MIN_OBS}",
            series.name,
            r.len(),
            freq.name()
        ));
    }
    let dt = (days[days.len() - 1] - days[0]) as f64 / 365.25 / (r.len() - 1) as f64;
    let mut gaps: Vec<i64> = days.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_unstable();
    let (median, max) = (gaps[gaps.len() / 2], gaps[gaps.len() - 1]);
    let mut warnings = Vec::new();
    if max > 6 * median.max(1) {
        warnings.push(format!(
            "the series has a gap of {max} days (median spacing {median} days); the fit assumes equal spacing"
        ));
    }
    Ok(Sample { series: series.name.clone(), freq, days, r, dt, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(t: &str) -> i64 {
        dates::parse(t).unwrap()
    }

    #[test]
    fn months_and_spans() {
        assert_eq!(dates::format(add_months(d("2020-01-31"), 1)), "2020-02-29");
        assert_eq!(dates::format(add_months(d("2020-11-15"), 3)), "2021-02-15");
        assert_eq!(dates::format(Span::parse("5y").unwrap().add_to(d("2000-03-01"))), "2005-03-01");
        assert_eq!(dates::format(Span::parse("2w").unwrap().add_to(d("2000-03-01"))), "2000-03-15");
        assert!(Span::parse("0y").is_err() && Span::parse("y").is_err() && Span::parse("5q").is_err());
    }

    fn business_days(n: usize) -> Series {
        let mut days = Vec::new();
        let mut day = d("2000-01-03"); // a Monday
        while days.len() < n {
            let wd = (day + 3).rem_euclid(7); // 0 = Monday
            if wd < 5 {
                days.push(day);
            }
            day += 1;
        }
        let values = (0..n).map(|i| 0.05 + 0.0001 * i as f64).collect();
        Series::new("test", days, values).unwrap()
    }

    #[test]
    fn weekly_and_monthly_take_the_last_observation() {
        let s = business_days(900); // 180 weeks
        let w = sample(&s, None, None, Freq::Weekly).unwrap();
        assert_eq!(w.r.len(), 180);
        // each weekly value is the Friday value: indices 4, 9, 14, ...
        assert!((w.r[0] - s.values[4]).abs() < 1e-15 && (w.r[1] - s.values[9]).abs() < 1e-15);
        let m = sample(&s, None, None, Freq::Monthly).unwrap();
        assert!(m.r.len() >= 41 && m.r.len() <= 43, "{}", m.r.len());
        // the monthly value is the last business day of January 2000: 2000-01-31
        assert_eq!(dates::format(m.days[0]), "2000-01-31");
        assert!((w.dt - 7.0 / 365.25).abs() < 1e-4);
    }

    #[test]
    fn interval_and_short_series() {
        let s = business_days(300);
        let from = s.days[100];
        let to = s.days[199];
        let sub = sample(&s, Some(from), Some(to), Freq::Daily).unwrap();
        assert_eq!(sub.r.len(), 100);
        let err = sample(&s, Some(from), Some(s.days[110]), Freq::Daily).unwrap_err();
        assert!(err.contains("at least 30"), "{err}");
    }
}
