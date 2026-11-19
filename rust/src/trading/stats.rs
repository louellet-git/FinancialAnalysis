//! Sums, means and standard deviations that add in the same order as numpy (pairwise
//! summation), so a mean of the Rust code is the same as the mean of the Python code to the
//! last digits.

/// numpy `add.reduce` for a contiguous float array.
pub fn sum(a: &[f64]) -> f64 {
    0.0 + pairwise(a)
}

fn pairwise(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for &x in a {
            res += x;
        }
        res
    } else if n <= 128 {
        let mut r = [a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]];
        let mut i = 8;
        while i < n - n % 8 {
            for (j, slot) in r.iter_mut().enumerate() {
                *slot += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise(&a[..n2]) + pairwise(&a[n2..])
    }
}

/// `a.mean()` of numpy and pandas.
pub fn mean(a: &[f64]) -> f64 {
    sum(a) / a.len() as f64
}

/// `a.std(ddof=ddof)` of numpy and pandas (NaN for an empty slice).
pub fn std(a: &[f64], ddof: usize) -> f64 {
    let n = a.len();
    let avg = sum(a) / n as f64;
    let sq: Vec<f64> = a.iter().map(|&x| (avg - x) * (avg - x)).collect();
    (sum(&sq) / (n as f64 - ddof as f64)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_statistics() {
        let a = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert_eq!(sum(&a), 40.0);
        assert_eq!(mean(&a), 5.0);
        assert_eq!(std(&a, 0), 2.0);
        assert!((std(&a, 1) - 2.138_089_935_299_395).abs() < 1e-12);
        let long: Vec<f64> = (0..1000).map(|x| x as f64).collect();
        assert_eq!(sum(&long), 499_500.0);
        assert!(std(&[], 0).is_nan());
    }
}
