//! s13 - Pre-FOMC announcement drift (Lucca and Moench 2015) with a call debit spread.
//!
//! Equities have earned a large part of their premium in the ~24 hours before a scheduled FOMC
//! statement (14:00 ET). With daily bars the code holds from the OPEN of the session before the
//! announcement day to the CLOSE of the announcement day. The dates are the SCHEDULED statement
//! days (published a year ahead, so no lookahead). Unscheduled actions (2008-01-22, 2008-10-08,
//! 2020-03-03, 2020-03-15) and the cancelled 2020-03-18 meeting are left out on purpose.

use crate::trading::calendar::{civil_from_days, days_from_civil, Date};
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;
use std::sync::OnceLock;

const FOMC: &str = "2005 0202 0322 0503 0630 0809 0920 1101 1213
2006 0131 0328 0510 0629 0808 0920 1025 1212
2007 0131 0321 0509 0628 0807 0918 1031 1211
2008 0130 0318 0430 0625 0805 0916 1029 1216
2009 0128 0318 0429 0624 0812 0923 1104 1216
2010 0127 0316 0428 0623 0810 0921 1103 1214
2011 0126 0315 0427 0622 0809 0921 1102 1213
2012 0125 0313 0425 0620 0801 0913 1024 1212
2013 0130 0320 0501 0619 0731 0918 1030 1218
2014 0129 0319 0430 0618 0730 0917 1029 1217
2015 0128 0318 0429 0617 0729 0917 1028 1216
2016 0127 0316 0427 0615 0727 0921 1102 1214
2017 0201 0315 0503 0614 0726 0920 1101 1213
2018 0131 0321 0502 0613 0801 0926 1108 1219
2019 0130 0320 0501 0619 0731 0918 1030 1211
2020 0129 0429 0610 0729 0916 1105 1216
2021 0127 0317 0428 0616 0728 0922 1103 1215
2022 0126 0316 0504 0615 0727 0921 1102 1214
2023 0201 0322 0503 0614 0726 0920 1101 1213
2024 0131 0320 0501 0612 0731 0918 1107 1218
2025 0129 0319 0507 0618 0730 0917 1029 1210
2026 0128 0318 0429 0617 0729 0916 1028 1209";

fn fomc_dates() -> &'static Vec<Date> {
    static DATES: OnceLock<Vec<Date>> = OnceLock::new();
    DATES.get_or_init(|| {
        let mut v = Vec::new();
        for line in FOMC.lines() {
            let mut it = line.split_whitespace();
            let y: i32 = it.next().and_then(|s| s.parse().ok()).expect("year");
            for d in it {
                let m: u32 = d[..2].parse().expect("month");
                let day: u32 = d[2..].parse().expect("day");
                v.push(days_from_civil(y, m, day));
            }
        }
        v.sort_unstable();
        v
    })
}

const DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const EXTRA_DTE: i32 = 1;
const MIN_YEAR: i32 = 2005;
const SKIP_VIX_ABOVE: f64 = 1.0;
const SPY: &str = "SPY";

#[derive(Default)]
pub struct PreFomcDrift {
    exit_on: Option<Date>,
}

impl Strategy for PreFomcDrift {
    fn name(&self) -> &'static str {
        "s13_fomc_drift"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-01")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        // Exit first: the close of the announcement day.
        for p in ctx.positions() {
            if ctx.today() >= self.exit_on.unwrap_or(ctx.today()) {
                ctx.close(&p, "fomc_day_close");
            }
        }
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        // The decision is at the close of D-2, and the fill is at the open of D-1. D = this day + 2 sessions.
        let d = ctx.expiry(2);
        if fomc_dates().binary_search(&d).is_err() || civil_from_days(d).0 < MIN_YEAR {
            return Ok(());
        }
        if ctx.vol(SPY).atm >= SKIP_VIX_ABOVE {
            return Ok(());
        }
        let exp = ctx.expiry(2 + EXTRA_DTE);
        let k = ctx.strike_for_delta(SPY, true, DELTA, exp, 1.0);
        self.exit_on = Some(d);
        ctx.open(vertical(SPY, exp, true, k, k + WIDTH)?, ExitRules::default(), "fomc", None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_dates_only() {
        let d = fomc_dates();
        assert_eq!(d.len(), 175);
        assert!(d.binary_search(&days_from_civil(2026, 12, 9)).is_ok());
        // The emergency actions and the cancelled meeting are not in the list.
        for (y, m, day) in [(2008, 1, 22), (2008, 10, 8), (2020, 3, 3), (2020, 3, 15), (2020, 3, 18)] {
            assert!(d.binary_search(&days_from_civil(y, m, day)).is_err());
        }
    }
}
