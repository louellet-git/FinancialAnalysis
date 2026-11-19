//! Number and text formats of the report. They give the same text as the Python format
//! specifications of the old script (`{:,.1f}`, `{:+.0%}`, `{:,.4g}`, `{:g}`, `html.escape`),
//! so a table cell has the same text.

/// `html.escape(text, quote=True)`.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

/// Put a comma between each group of three digits of the integer part (`text` has an optional sign).
pub fn group(text: &str) -> String {
    let (sign, rest) = match text.strip_prefix(['-', '+']) {
        Some(r) => (&text[..1], r),
        None => ("", text),
    };
    let (int, frac) = match rest.find('.') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut out = String::from(sign);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out.push_str(frac);
    out
}

/// `f"{v:.{dec}f}"`.
pub fn fixed(v: f64, dec: usize) -> String {
    format!("{v:.dec$}")
}

/// `f"{v:,.{dec}f}"`.
pub fn fixed_c(v: f64, dec: usize) -> String {
    group(&fixed(v, dec))
}

/// `f"{v:+.{dec}f}"`.
pub fn signed(v: f64, dec: usize) -> String {
    format!("{v:+.dec$}")
}

/// `f"{v:+,.{dec}f}"`.
pub fn signed_c(v: f64, dec: usize) -> String {
    group(&signed(v, dec))
}

/// `f"{v:.{dec}%}"` (Python multiplies by 100 as a float, then formats).
pub fn pct(v: f64, dec: usize) -> String {
    format!("{:.dec$}%", v * 100.0)
}

/// `f"{v:+.{dec}%}"`.
pub fn pct_signed(v: f64, dec: usize) -> String {
    format!("{:+.dec$}%", v * 100.0)
}

/// `f"{v:.{prec}g}"` without the grouping. `prec` is at least 1.
pub fn general(v: f64, prec: usize) -> String {
    if v == 0.0 {
        return if v.is_sign_negative() { "-0".to_string() } else { "0".to_string() };
    }
    let sci = format!("{:.*e}", prec - 1, v);
    let (mant, exp) = sci.split_once('e').expect("exponent form");
    let exp: i32 = exp.parse().expect("exponent");
    if exp < -4 || exp >= prec as i32 {
        let mant = if mant.contains('.') { mant.trim_end_matches('0').trim_end_matches('.') } else { mant };
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{mant}e{sign}{:02}", exp.abs());
    }
    let dec = (prec as i32 - 1 - exp).max(0) as usize;
    let fixed = format!("{v:.dec$}");
    if fixed.contains('.') {
        fixed.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        fixed
    }
}

/// `f"{v:,.4g}"` (the `_fmt` of the driver table).
pub fn general_c(v: f64, prec: usize) -> String {
    group(&general(v, prec))
}

/// `f"{v:g}"` (six digits).
pub fn g(v: f64) -> String {
    general(v, 6)
}

/// `f"{n:,}"`.
pub fn int_c(n: i64) -> String {
    group(&n.to_string())
}

/// `round(x, digits)` of Python: the nearest decimal, a tie to the even digit (exact binary value).
pub fn pyround(x: f64, digits: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.digits$}").parse().unwrap_or(x)
}

/// `np.round(x, digits)`: `rint(x * 10^d) / 10^d`.
pub fn npround(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (x * f).round_ties_even() / f
}

/// `"${v/1e9:,.{dec}f}B"`, or the dash for a missing value.
pub fn money_b(v: Option<f64>, dec: usize) -> String {
    match v {
        None => "\u{2014}".to_string(),
        Some(v) => format!("${}B", fixed_c(v / 1e9, dec)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_formats() {
        assert_eq!(fixed_c(1234567.891, 1), "1,234,567.9");
        assert_eq!(fixed_c(-1234.5, 0), "-1,234");
        assert_eq!(fixed_c(0.5, 0), "0");
        assert_eq!(fixed_c(1.5, 0), "2");
        assert_eq!(signed_c(-29.4, 0), "-29");
        assert_eq!(signed(0.0, 1), "+0.0");
        assert_eq!(pct(0.1791959724587251, 1), "17.9%");
        assert_eq!(pct_signed(-0.553, 0), "-55%");
        assert_eq!(general_c(0.0215, 4), "0.0215");
        assert_eq!(general_c(0.04, 4), "0.04");
        assert_eq!(general_c(1234.5, 4), "1,234");
        assert_eq!(general_c(12345.0, 4), "1.234e+04");
        assert_eq!(general_c(0.00001234, 4), "1.234e-05");
        assert_eq!(general_c(100.0, 4), "100");
        assert_eq!(general_c(1.5, 4), "1.5");
        assert_eq!(general_c(0.0, 4), "0");
        assert_eq!(g(225.0), "225");
        assert_eq!(g(97.5), "97.5");
        assert_eq!(g(1000000.0), "1e+06");
        assert_eq!(int_c(1234567), "1,234,567");
        assert_eq!(esc("a & b <c> \"d\" 'e'"), "a &amp; b &lt;c&gt; &quot;d&quot; &#x27;e&#x27;");
        assert_eq!(pyround(0.125, 2), 0.12);
        assert_eq!(pyround(2.675, 2), 2.67);
        assert_eq!(npround(2.5, 0), 2.0);
        assert_eq!(npround(3.5, 0), 4.0);
        assert_eq!(money_b(None, 1), "\u{2014}");
        assert_eq!(money_b(Some(-2.5e9), 1), "$-2.5B");
    }
}
