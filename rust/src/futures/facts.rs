//! Contract facts for the roots that this program supports.
//!
//! The facts come from the public contract specifications of the exchanges. The CME
//! website refuses automated access, so the program cannot test them against the
//! exchange page. Check a fact before you use it for money.

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Root {
    pub root: &'static str,
    pub name: &'static str,
    pub exchange: &'static str,
    /// Yahoo symbol suffix for a single contract, for example ".CME". Empty: no Yahoo data.
    pub yahoo_suffix: &'static str,
    /// Contract month letters that this program lists.
    pub months: &'static str,
    pub tick: &'static str,
    pub price_means: &'static str,
    /// Text for the rate, or None when the price gives no direct rate.
    pub implied_rate: Option<&'static str>,
    /// CFTC Commitments of Traders code (Traders in Financial Futures report).
    pub cftc_code: &'static str,
}

const QUARTERLY: &str = "HMUZ";
const MONTHLY: &str = "FGHJKMNQUVXZ";

pub const ROOTS: [Root; 12] = [
    Root { root: "ES", name: "E-mini S&P 500", exchange: "CME", yahoo_suffix: ".CME", months: QUARTERLY,
        tick: "0.25 index points (USD 12.50)", price_means: "S&P 500 index level; USD 50 for each point",
        implied_rate: None, cftc_code: "13874A" },
    Root { root: "NQ", name: "E-mini Nasdaq-100", exchange: "CME", yahoo_suffix: ".CME", months: QUARTERLY,
        tick: "0.25 index points (USD 5.00)", price_means: "Nasdaq-100 index level; USD 20 for each point",
        implied_rate: None, cftc_code: "209742" },
    Root { root: "ZT", name: "2-Year T-Note", exchange: "CBOT", yahoo_suffix: ".CBT", months: QUARTERLY,
        tick: "1/8 of 1/32 point (0.00390625)", price_means: "Price of a notional 6% US Treasury note, 1.75 to 2 years to maturity, face USD 200,000",
        implied_rate: None, cftc_code: "042601" },
    Root { root: "ZF", name: "5-Year T-Note", exchange: "CBOT", yahoo_suffix: ".CBT", months: QUARTERLY,
        tick: "1/4 of 1/32 point (0.0078125)", price_means: "Price of a notional 6% US Treasury note, face USD 100,000",
        implied_rate: None, cftc_code: "044601" },
    Root { root: "ZN", name: "10-Year T-Note", exchange: "CBOT", yahoo_suffix: ".CBT", months: QUARTERLY,
        tick: "1/2 of 1/32 point (0.015625)", price_means: "Price of a notional 6% US Treasury note, 6.5 to 8 years to maturity, face USD 100,000",
        implied_rate: None, cftc_code: "043602" },
    Root { root: "TN", name: "Ultra 10-Year T-Note", exchange: "CBOT", yahoo_suffix: ".CBT", months: QUARTERLY,
        tick: "1/2 of 1/32 point (0.015625)", price_means: "Price of a notional 6% US Treasury note, 9.5 to 10 years to maturity, face USD 100,000",
        implied_rate: None, cftc_code: "043607" },
    Root { root: "ZB", name: "U.S. Treasury Bond", exchange: "CBOT", yahoo_suffix: ".CBT", months: QUARTERLY,
        tick: "1/32 point (0.03125)", price_means: "Price of a notional 6% US Treasury bond, 15 to 25 years to maturity, face USD 100,000",
        implied_rate: None, cftc_code: "020601" },
    Root { root: "UB", name: "Ultra U.S. Treasury Bond", exchange: "CBOT", yahoo_suffix: ".CBT", months: QUARTERLY,
        tick: "1/32 point (0.03125)", price_means: "Price of a notional 6% US Treasury bond, 25 years or more to maturity, face USD 100,000",
        implied_rate: None, cftc_code: "020604" },
    Root { root: "SR3", name: "3-Month SOFR", exchange: "CME", yahoo_suffix: ".CME", months: QUARTERLY,
        tick: "0.0025 (nearest quarterly) or 0.005 (others); 1 basis point = USD 25",
        price_means: "100 minus the compounded average of the daily SOFR over the 3-month reference quarter that starts on the third Wednesday of the contract month",
        implied_rate: Some("(100 - price) / 100"), cftc_code: "134741" },
    Root { root: "SR1", name: "1-Month SOFR", exchange: "CME", yahoo_suffix: ".CME", months: MONTHLY,
        tick: "0.0025 (nearest month) or 0.005 (others); 1 basis point = USD 41.67",
        price_means: "100 minus the average of the daily SOFR over the contract month",
        implied_rate: Some("(100 - price) / 100"), cftc_code: "134742" },
    Root { root: "ZQ", name: "30-Day Fed Funds", exchange: "CBOT", yahoo_suffix: ".CBT", months: MONTHLY,
        tick: "0.0025 (nearest month) or 0.005 (others); 1 basis point = USD 41.67",
        price_means: "100 minus the average of the daily effective federal funds rate (EFFR) over the contract month",
        implied_rate: Some("(100 - price) / 100"), cftc_code: "045601" },
    Root { root: "VX", name: "Cboe Volatility Index (VIX) futures", exchange: "CFE", yahoo_suffix: "", months: MONTHLY,
        tick: "0.05 index points (USD 50)", price_means: "Expected VIX level at expiry; the final settlement is the VIX opening value (SOQ) of the expiry day; USD 1,000 for each point",
        implied_rate: None, cftc_code: "1170E1" },
];

pub fn root(name: &str) -> Option<&'static Root> {
    ROOTS.iter().find(|r| r.root.eq_ignore_ascii_case(name))
}

pub fn root_or_error(name: &str) -> Result<&'static Root, String> {
    root(name).ok_or_else(|| {
        format!("unknown root '{name}'; the roots are: {}", ROOTS.iter().map(|r| r.root).collect::<Vec<_>>().join(", "))
    })
}

/// "F" -> 1 ... "Z" -> 12.
pub fn month_number(letter: char) -> Option<u32> {
    MONTHLY.find(letter).map(|i| i as u32 + 1)
}

pub fn month_letter(month: u32) -> char {
    MONTHLY.as_bytes()[month as usize - 1] as char
}

/// A contract: a root and a delivery month.
#[derive(Debug, Clone, PartialEq)]
pub struct Contract {
    pub root: &'static Root,
    pub month: u32,
    pub year: i64,
}

impl Contract {
    /// "ESZ26"
    pub fn code(&self) -> String {
        format!("{}{}{:02}", self.root.root, month_letter(self.month), self.year % 100)
    }

    /// "2026-12"
    pub fn label(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }
}

/// What the user typed: a root alone (continuous series) or a contract such as ESZ26.
#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Continuous(&'static Root),
    Contract(Contract),
}

pub fn parse_symbol(text: &str) -> Result<Symbol, String> {
    let upper = text.trim().to_ascii_uppercase();
    // Longest root first, so SR3 is not read as another root.
    let mut roots: Vec<&Root> = ROOTS.iter().collect();
    roots.sort_by_key(|r| std::cmp::Reverse(r.root.len()));
    for r in roots {
        let Some(rest) = upper.strip_prefix(r.root) else { continue };
        if rest.is_empty() {
            return Ok(Symbol::Continuous(r));
        }
        let chars: Vec<char> = rest.chars().collect();
        if chars.len() == 3 && chars[1].is_ascii_digit() && chars[2].is_ascii_digit() {
            if let Some(month) = month_number(chars[0]) {
                if !r.months.contains(chars[0]) {
                    return Err(format!("{} has no contract in month {}; the months are {}", r.root, chars[0], r.months));
                }
                let year = 2000 + rest[1..].parse::<i64>().map_err(|_| format!("bad year in '{text}'"))?;
                return Ok(Symbol::Contract(Contract { root: r, month, year }));
            }
        }
    }
    let roots: Vec<&str> = ROOTS.iter().map(|r| r.root).collect();
    Err(format!("bad symbol '{text}': use a root (ES) or a contract (ESZ26); the roots are: {}", roots.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols() {
        assert!(matches!(parse_symbol("es").unwrap(), Symbol::Continuous(r) if r.root == "ES"));
        match parse_symbol("SR3Z26").unwrap() {
            Symbol::Contract(c) => assert_eq!((c.root.root, c.month, c.year, c.code().as_str()), ("SR3", 12, 2026, "SR3Z26")),
            other => panic!("{other:?}"),
        }
        match parse_symbol("SR1F27").unwrap() {
            Symbol::Contract(c) => assert_eq!(c.label(), "2027-01"),
            other => panic!("{other:?}"),
        }
        assert!(parse_symbol("ESF26").unwrap_err().contains("no contract in month F"));
        assert!(parse_symbol("XYZ").unwrap_err().contains("the roots are"));
        assert!(parse_symbol("ESZ2").is_err());
        assert!(parse_symbol("").is_err());
    }
}
