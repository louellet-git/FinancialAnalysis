//! The data sets of `fa sync`: their names, maximum ages, hosts and request pauses. One table for each.

/// One data set. The names are the names of `--only` and `--skip` and of the table in `docs/DATA-SOURCES.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataSet {
    Sec,
    Filings,
    Prices,
    Intraday,
    Estimates,
    Earnings,
    Options,
    Rates,
    Gsw,
    Fred,
    Futures,
    Vix,
    Bonds,
}

/// All data sets, in the order of a run (the SEC data sets first: the other data sets of the SEC host come after them).
pub const ALL: [DataSet; 13] = [
    DataSet::Sec,
    DataSet::Filings,
    DataSet::Prices,
    DataSet::Intraday,
    DataSet::Estimates,
    DataSet::Earnings,
    DataSet::Options,
    DataSet::Rates,
    DataSet::Gsw,
    DataSet::Fred,
    DataSet::Futures,
    DataSet::Vix,
    DataSet::Bonds,
];

/// What a data set is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// One item for each ticker of the list.
    Tickers,
    /// One item for each symbol of a list of the config (intraday, options).
    Symbols,
    /// A few fixed items.
    Fixed,
}

/// How the age is counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeUnit {
    /// Market days after the date of the data (a weekend or a market holiday adds nothing).
    MarketDays,
    /// Calendar days.
    CalendarDays,
}

impl DataSet {
    pub fn name(self) -> &'static str {
        match self {
            DataSet::Sec => "sec",
            DataSet::Filings => "filings",
            DataSet::Prices => "prices",
            DataSet::Intraday => "intraday",
            DataSet::Estimates => "estimates",
            DataSet::Earnings => "earnings",
            DataSet::Options => "options",
            DataSet::Rates => "rates",
            DataSet::Gsw => "gsw",
            DataSet::Fred => "fred",
            DataSet::Futures => "futures",
            DataSet::Vix => "vix",
            DataSet::Bonds => "bonds",
        }
    }

    pub fn from_name(name: &str) -> Option<DataSet> {
        ALL.iter().copied().find(|d| d.name() == name.trim().to_lowercase())
    }

    pub fn scope(self) -> Scope {
        match self {
            DataSet::Sec | DataSet::Filings | DataSet::Prices | DataSet::Estimates | DataSet::Earnings => Scope::Tickers,
            DataSet::Intraday | DataSet::Options => Scope::Symbols,
            DataSet::Rates | DataSet::Gsw | DataSet::Fred | DataSet::Futures | DataSet::Vix | DataSet::Bonds => Scope::Fixed,
        }
    }

    /// The default maximum age, in days of the unit of the data set. A data set is old when its age is more than this
    /// (calendar days) or when its age reaches this (market days: the age 0 is the same session).
    pub fn default_max_age(self) -> u32 {
        match self {
            DataSet::Estimates => 7,
            _ => 1,
        }
    }

    pub fn age_unit(self) -> AgeUnit {
        match self {
            DataSet::Estimates => AgeUnit::CalendarDays,
            _ => AgeUnit::MarketDays,
        }
    }

    /// The hosts of the data set, for the table of the documents (the first is the first source).
    pub fn hosts(self) -> &'static [&'static str] {
        match self {
            DataSet::Sec | DataSet::Filings => &["sec"],
            DataSet::Prices => &["tiingo", "alpaca", "yahoo", "nasdaq"],
            DataSet::Intraday => &["alpaca", "yahoo"],
            DataSet::Estimates => &["nasdaq"],
            DataSet::Earnings => &["nasdaq", "sec"],
            DataSet::Options => &["cboe", "alpaca"],
            DataSet::Rates => &["treasury", "bls"],
            DataSet::Gsw => &["federalreserve"],
            DataSet::Fred => &["fred"],
            DataSet::Futures => &["cboe", "atlantafed", "cftc", "yahoo"],
            DataSet::Vix => &["cboe"],
            DataSet::Bonds => &["treasurydirect"],
        }
    }

    /// Does the data set need the SEC contact text (`SEC_USER_AGENT`)?
    pub fn needs_sec_contact(self) -> bool {
        matches!(self, DataSet::Sec | DataSet::Filings)
    }
}

/// The pause between two requests to one host, and the time that one request takes (for the estimate of `--dry-run`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostSpec {
    pub name: &'static str,
    /// Seconds between two requests to the host: the pause of the existing client of that host.
    pub pause: f64,
    /// Seconds that one request takes, on average (an estimate).
    pub request_time: f64,
}

/// The hosts of `fa sync` with the pauses of the existing clients. The SEC allows 10 requests each second; the existing
/// client waits 0.15 seconds. `futures::DELAY_SECONDS` is 0.5 (Cboe, CFTC, Atlanta Fed, Yahoo for futures); the market
/// module uses 2 seconds for Nasdaq, Yahoo, Tiingo and Cboe option files; Alpaca 0.4; FedInvest 1.
pub const HOSTS: [HostSpec; 15] = [
    HostSpec { name: "sec", pause: 0.15, request_time: 0.4 },
    HostSpec { name: "nasdaq", pause: 2.0, request_time: 0.6 },
    HostSpec { name: "alpaca", pause: 0.4, request_time: 0.5 },
    HostSpec { name: "tiingo", pause: 2.0, request_time: 0.5 },
    HostSpec { name: "yahoo", pause: 2.0, request_time: 0.6 },
    HostSpec { name: "cboe", pause: 2.0, request_time: 0.8 },
    HostSpec { name: "treasury", pause: 2.0, request_time: 1.0 },
    HostSpec { name: "nyfed", pause: 2.0, request_time: 0.6 },
    HostSpec { name: "fred", pause: 2.0, request_time: 0.8 },
    HostSpec { name: "federalreserve", pause: 0.2, request_time: 2.0 },
    HostSpec { name: "bls", pause: 2.0, request_time: 0.6 },
    HostSpec { name: "treasurydirect", pause: 1.0, request_time: 3.0 },
    HostSpec { name: "fiscaldata", pause: 2.0, request_time: 0.6 },
    HostSpec { name: "cftc", pause: 0.5, request_time: 0.8 },
    HostSpec { name: "atlantafed", pause: 0.5, request_time: 1.5 },
];

pub fn host_spec(name: &str) -> HostSpec {
    HOSTS.iter().copied().find(|h| h.name == name).unwrap_or(HostSpec { name: "other", pause: 2.0, request_time: 1.0 })
}

/// The seconds that `n` requests to a host take: the request time and the pause for each of them.
pub fn host_seconds(host: &str, n: usize) -> f64 {
    let h = host_spec(host);
    n as f64 * (h.request_time + h.pause)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_ages_are_in_one_table() {
        for d in ALL {
            assert_eq!(DataSet::from_name(d.name()), Some(d));
            assert!(d.default_max_age() >= 1);
            assert!(!d.hosts().is_empty());
        }
        assert_eq!(DataSet::from_name(" Prices "), Some(DataSet::Prices));
        assert_eq!(DataSet::from_name("nope"), None);
        assert_eq!(DataSet::Estimates.default_max_age(), 7);
        assert_eq!(DataSet::Estimates.age_unit(), AgeUnit::CalendarDays);
        assert_eq!(DataSet::Prices.age_unit(), AgeUnit::MarketDays);
    }

    #[test]
    fn every_host_of_a_data_set_has_a_pause() {
        for d in ALL {
            for h in d.hosts() {
                assert!(HOSTS.iter().any(|s| s.name == *h), "{h} of {}", d.name());
            }
        }
        // The pauses of the existing clients.
        assert_eq!(host_spec("sec").pause, 0.15);
        assert_eq!(host_spec("nasdaq").pause, 2.0);
        assert!((host_seconds("nasdaq", 10) - 26.0).abs() < 1e-9);
    }
}
