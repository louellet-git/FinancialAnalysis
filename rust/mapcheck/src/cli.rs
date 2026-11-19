//! A small reader for command-line arguments.

use std::collections::HashMap;

pub struct Args {
    values: HashMap<String, String>,
    flags: Vec<String>,
    pub positional: Vec<String>,
}

impl Args {
    /// `with_value`: options that take a value. `switches`: options with no value.
    pub fn parse(args: &[String], with_value: &[&str], switches: &[&str]) -> Result<Args, String> {
        let mut out = Args { values: HashMap::new(), flags: Vec::new(), positional: Vec::new() };
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if with_value.contains(&arg.as_str()) {
                let value = iter.next().ok_or_else(|| format!("{arg} needs a value"))?;
                out.values.insert(arg.clone(), value.clone());
            } else if switches.contains(&arg.as_str()) {
                out.flags.push(arg.clone());
            } else if arg.starts_with('-') && arg.len() > 1 {
                return Err(format!("unknown option '{arg}'"));
            } else {
                out.positional.push(arg.clone());
            }
        }
        Ok(out)
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }

    pub fn number<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        match self.value(name) {
            None => Ok(default),
            Some(text) => text.parse().map_err(|_| format!("{name} needs a number, not '{text}'")),
        }
    }

    /// The tickers: the positional arguments and the lines of --tickers-file, upper case, no duplicates.
    pub fn tickers(&self) -> Result<Vec<String>, String> {
        let mut out: Vec<String> = self.positional.iter().map(|t| t.to_uppercase()).collect();
        if let Some(path) = self.value("--tickers-file") {
            out.extend(read_ticker_file(path)?);
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|t| seen.insert(t.clone()));
        Ok(out)
    }
}

/// One ticker for each line; `#` starts a comment. A line can have the SEC company
/// number (CIK) after the ticker: `IBM 51143`.
pub fn read_ticker_file(path: &str) -> Result<Vec<String>, String> {
    Ok(read_ticker_ciks(path)?.into_iter().map(|(ticker, _)| ticker).collect())
}

/// (ticker, CIK if the line has one) for each line of a ticker file.
pub fn read_ticker_ciks(path: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut out = Vec::new();
    for line in text.lines() {
        let mut words = line.split('#').next().unwrap_or("").split_whitespace();
        let Some(ticker) = words.next() else { continue };
        let cik = words.next().map(|c| c.trim_start_matches('0').to_string()).filter(|c| !c.is_empty());
        if let Some(cik) = &cik {
            if !cik.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("{path}: '{cik}' after {ticker} is not a CIK number"));
            }
        }
        out.push((ticker.to_uppercase(), cik));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_switches_and_tickers() {
        let args = Args::parse(&strings(&["ibm", "--cache", "x", "--list", "jnj", "IBM"]), &["--cache"], &["--list"]).unwrap();
        assert_eq!(args.value("--cache"), Some("x"));
        assert!(args.flag("--list") && !args.flag("--detail"));
        assert_eq!(args.tickers().unwrap(), vec!["IBM", "JNJ"]);
        assert_eq!(args.number("--top", 25usize).unwrap(), 25);
    }

    #[test]
    fn ticker_file_can_have_a_cik_after_the_ticker() {
        let path = std::env::temp_dir().join(format!("mapcheck-cli-test-{}.txt", std::process::id()));
        std::fs::write(&path, "# list\nibm 0000051143  # Information Technology\nBRK-B 1067983\n\nmcd\n").unwrap();
        let path_text = path.to_string_lossy().to_string();
        assert_eq!(read_ticker_file(&path_text).unwrap(), vec!["IBM", "BRK-B", "MCD"]);
        let ciks = read_ticker_ciks(&path_text).unwrap();
        assert_eq!(ciks[0], ("IBM".to_string(), Some("51143".to_string())));
        assert_eq!(ciks[2], ("MCD".to_string(), None));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unknown_option_and_missing_value_are_errors() {
        assert!(Args::parse(&strings(&["--nope"]), &[], &[]).is_err());
        assert!(Args::parse(&strings(&["--cache"]), &["--cache"], &[]).is_err());
    }
}
