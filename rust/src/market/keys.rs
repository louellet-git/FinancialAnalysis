//! Keys of the keyed data sources: read from the environment, else from `<repo>/.env`, never printed.
//!
//! Key names: `TIINGO_API_KEY`, `FMP_API_KEY`, `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY` (Alpaca),
//! `FRED_API_KEY`. The names are in `.env.example`; the values are not in git. A text that goes to a
//! person (an error, a log line) passes through `scrub_all` first: each key value becomes `***`.

use std::path::Path;

/// The value of `name` in the text of a `.env` file (`NAME=value`, `export NAME=value`, quotes allowed).
pub fn env_file_value(text: &str, name: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some(value) = line.strip_prefix(name).and_then(|r| r.trim_start().strip_prefix('=')) {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// The value of the environment variable, else of the `.env` file in `repo`.
pub fn read_key(repo: &Path, name: &str) -> Option<String> {
    if let Ok(v) = std::env::var(name) {
        if !v.trim().is_empty() {
            return Some(v.trim().to_string());
        }
    }
    env_file_value(&std::fs::read_to_string(repo.join(".env")).ok()?, name)
}

/// The text with each secret taken out. A secret shorter than 4 characters is not replaced (it would
/// damage the text); real keys are longer.
pub fn scrub_all(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_string();
    for s in secrets {
        if s.len() >= 4 {
            out = out.replace(s, "***");
        }
    }
    out
}

/// The text with the value of each `api_key=` parameter taken out (the FRED key is a URL parameter, so
/// the URL in an error text has it). Works with no knowledge of the key.
pub fn scrub_api_key_param(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("api_key=") {
        let (head, tail) = rest.split_at(i + "api_key=".len());
        out.push_str(head);
        let end = tail.find(|c: char| c == '&' || c.is_whitespace() || c == '"' || c == '\'' || c == ')').unwrap_or(tail.len());
        out.push_str("***");
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// Percent-encoding of a URL query value (the unreserved characters stay).
pub fn urlenc(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_file_values() {
        let text = "A=1\nexport APCA_API_KEY_ID=\"abc123\"\nAPCA_API_SECRET_KEY = 'sec' \nEMPTY=\n";
        assert_eq!(env_file_value(text, "APCA_API_KEY_ID").as_deref(), Some("abc123"));
        assert_eq!(env_file_value(text, "APCA_API_SECRET_KEY").as_deref(), Some("sec"));
        assert_eq!(env_file_value(text, "EMPTY"), None);
        assert_eq!(env_file_value(text, "FRED_API_KEY"), None);
    }

    #[test]
    fn scrubbing() {
        assert_eq!(scrub_all("bad KEYVALUE and SECRETVALUE", &["KEYVALUE", "SECRETVALUE", ""]), "bad *** and ***");
        assert_eq!(scrub_all("ab", &["ab"]), "ab");
        assert_eq!(scrub_api_key_param("HTTP 400 for https://x/y?series_id=A&api_key=abcdef0123&file_type=json"), "HTTP 400 for https://x/y?series_id=A&api_key=***&file_type=json");
        assert_eq!(scrub_api_key_param("https://x/y?api_key=zzz"), "https://x/y?api_key=***");
        assert_eq!(scrub_api_key_param("no key here"), "no key here");
    }

    #[test]
    fn url_encoding() {
        assert_eq!(urlenc("QUFQ=="), "QUFQ%3D%3D");
        assert_eq!(urlenc("a+b/c"), "a%2Bb%2Fc");
        assert_eq!(urlenc("AZaz09-_.~"), "AZaz09-_.~");
    }
}
