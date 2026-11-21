//! A small complex number type. The crate has no complex-number crate.

use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cx {
    pub re: f64,
    pub im: f64,
}

impl Cx {
    pub const ZERO: Cx = Cx { re: 0.0, im: 0.0 };
    pub const ONE: Cx = Cx { re: 1.0, im: 0.0 };
    pub const I: Cx = Cx { re: 0.0, im: 1.0 };

    #[inline]
    pub fn new(re: f64, im: f64) -> Cx {
        Cx { re, im }
    }
    #[inline]
    pub fn real(re: f64) -> Cx {
        Cx { re, im: 0.0 }
    }
    #[inline]
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    #[inline]
    pub fn conj(self) -> Cx {
        Cx::new(self.re, -self.im)
    }
    #[inline]
    pub fn scale(self, s: f64) -> Cx {
        Cx::new(self.re * s, self.im * s)
    }
    #[inline]
    pub fn exp(self) -> Cx {
        let m = self.re.exp();
        let (s, c) = self.im.sin_cos();
        Cx::new(m * c, m * s)
    }
    /// Square root with a non-negative real part. No cancellation for a small imaginary part.
    pub fn sqrt(self) -> Cx {
        let m = self.abs();
        if m == 0.0 {
            return Cx::ZERO;
        }
        if self.re >= 0.0 {
            let r = ((m + self.re) * 0.5).sqrt();
            Cx::new(r, self.im / (2.0 * r))
        } else {
            let s = ((m - self.re) * 0.5).sqrt();
            Cx::new(self.im.abs() / (2.0 * s), if self.im < 0.0 { -s } else { s })
        }
    }
    /// ln(1 + w) on the principal branch, accurate for a small `w`.
    pub fn ln1p(self) -> Cx {
        let n2 = self.re * self.re + self.im * self.im;
        Cx::new(0.5 * (2.0 * self.re + n2).ln_1p(), self.im.atan2(1.0 + self.re))
    }
    /// ln(1 + w) / w, with the limit 1 at w = 0.
    pub fn ln1p_over(self) -> Cx {
        if self.abs() < 1e-5 {
            let w = self;
            Cx::ONE - w.scale(0.5) + (w * w).scale(1.0 / 3.0) - (w * w * w).scale(0.25)
        } else {
            self.ln1p() / self
        }
    }
}

impl Add for Cx {
    type Output = Cx;
    #[inline]
    fn add(self, o: Cx) -> Cx {
        Cx::new(self.re + o.re, self.im + o.im)
    }
}
impl Sub for Cx {
    type Output = Cx;
    #[inline]
    fn sub(self, o: Cx) -> Cx {
        Cx::new(self.re - o.re, self.im - o.im)
    }
}
impl Mul for Cx {
    type Output = Cx;
    #[inline]
    fn mul(self, o: Cx) -> Cx {
        Cx::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}
impl Div for Cx {
    type Output = Cx;
    #[inline]
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        Cx::new((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }
}
impl Neg for Cx {
    type Output = Cx;
    #[inline]
    fn neg(self) -> Cx {
        Cx::new(-self.re, -self.im)
    }
}
impl Add<f64> for Cx {
    type Output = Cx;
    #[inline]
    fn add(self, o: f64) -> Cx {
        Cx::new(self.re + o, self.im)
    }
}
impl Sub<f64> for Cx {
    type Output = Cx;
    #[inline]
    fn sub(self, o: f64) -> Cx {
        Cx::new(self.re - o, self.im)
    }
}
impl Mul<f64> for Cx {
    type Output = Cx;
    #[inline]
    fn mul(self, o: f64) -> Cx {
        self.scale(o)
    }
}
