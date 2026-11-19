//! Random numbers: xoshiro256++ with one stream for each path.
//!
//! The stream of path `p` depends only on `(seed, p)`. So a simulation gives the same
//! numbers with any number of threads.

#[inline]
fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[derive(Debug, Clone)]
pub struct Rng {
    s: [u64; 4],
    spare: Option<f64>,
}

impl Rng {
    /// The stream of path `path` for the seed `seed`.
    pub fn for_path(seed: u64, path: u64) -> Rng {
        let mut x = seed;
        let base = splitmix64(&mut x);
        let mut y = base ^ path.wrapping_add(1).wrapping_mul(0xD1B5_4A32_D192_ED03);
        let s = [splitmix64(&mut y), splitmix64(&mut y), splitmix64(&mut y), splitmix64(&mut y)];
        Rng { s, spare: None }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[0].wrapping_add(self.s[3]).rotate_left(23).wrapping_add(self.s[0]);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform on the open interval (0, 1).
    #[inline]
    pub fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Standard normal (Box-Muller; each pair of uniforms gives two normals).
    #[inline]
    pub fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let (u1, u2) = (self.uniform(), self.uniform());
        let radius = (-2.0 * u1.ln()).sqrt();
        let (s, c) = (2.0 * std::f64::consts::PI * u2).sin_cos();
        self.spare = Some(radius * s);
        radius * c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_reproducible_and_different() {
        let mut a = Rng::for_path(7, 3);
        let mut b = Rng::for_path(7, 3);
        let mut c = Rng::for_path(7, 4);
        let (x, y, z) = (a.next_u64(), b.next_u64(), c.next_u64());
        assert_eq!(x, y);
        assert_ne!(x, z);
    }

    #[test]
    fn normal_moments() {
        let mut r = Rng::for_path(1, 0);
        let n = 400_000;
        let (mut s1, mut s2, mut s3, mut s4) = (0.0, 0.0, 0.0, 0.0);
        for _ in 0..n {
            let z = r.normal();
            s1 += z;
            s2 += z * z;
            s3 += z * z * z;
            s4 += z * z * z * z;
        }
        let n = n as f64;
        // standard errors: 1/sqrt(n) for the mean, sqrt(2/n) for the variance, sqrt(6/n), sqrt(24/n)
        assert!((s1 / n).abs() < 4.0 / n.sqrt());
        assert!((s2 / n - 1.0).abs() < 4.0 * (2.0 / n).sqrt());
        assert!((s3 / n).abs() < 4.0 * (6.0 / n).sqrt());
        assert!((s4 / n - 3.0).abs() < 4.0 * (96.0 / n).sqrt());
    }

    #[test]
    fn uniform_is_open() {
        let mut r = Rng::for_path(9, 9);
        for _ in 0..100_000 {
            let u = r.uniform();
            assert!(u > 0.0 && u < 1.0);
        }
    }
}
