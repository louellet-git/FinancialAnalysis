//! The random numbers of numpy, bit for bit.
//!
//! The Python engines used `np.random.default_rng(seed)`, `standard_normal` and
//! `normal`. Each published number depends on that stream. This module makes the same
//! stream: `SeedSequence` (integer seed) -> `PCG64` (XSL-RR 128/64) -> ziggurat
//! `standard_normal`.
//!
//! Sources, numpy v2.5.3 (BSD-3-Clause; PCG64 code: Apache-2.0 / MIT, M. O'Neill and
//! R. Kern; see THIRD-PARTY.md):
//! - `numpy/random/bit_generator.pyx` (`SeedSequence`, `hashmix`, `mix`, `generate_state`)
//! - `numpy/random/_pcg64.pyx`, `src/pcg64/pcg64.h`, `pcg64.c` (seeding, step, output)
//! - `numpy/random/src/distributions/distributions.c` (`random_standard_normal`)
//! - `ziggurat_constants.h` (tables: see `ziggurat_tables.rs`)

use super::ziggurat_tables::{FI_DOUBLE, KI_DOUBLE, WI_DOUBLE};

const INIT_A: u32 = 0x43b0_d7e5;
const MULT_A: u32 = 0x931e_8875;
const INIT_B: u32 = 0x8b51_f9dd;
const MULT_B: u32 = 0x58f3_8ded;
const MIX_MULT_L: u32 = 0xca01_f9dd;
const MIX_MULT_R: u32 = 0x4973_f715;
const XSHIFT: u32 = 16;
const POOL_SIZE: usize = 4;

const PCG_MULT: u128 = ((2549297995355413924u128) << 64) | 4865540595714422341u128;

#[allow(clippy::excessive_precision)] // the digits of numpy's distributions.c
pub(crate) const ZIGGURAT_NOR_R: f64 = 3.6541528853610087963519472518;
#[allow(clippy::excessive_precision)]
const ZIGGURAT_NOR_INV_R: f64 = 0.27366123732975827203338247596;

fn hashmix(value: u32, hash_const: &mut u32) -> u32 {
    let mut v = value ^ *hash_const;
    *hash_const = hash_const.wrapping_mul(MULT_A);
    v = v.wrapping_mul(*hash_const);
    v ^ (v >> XSHIFT)
}

fn mix(x: u32, y: u32) -> u32 {
    let r = MIX_MULT_L.wrapping_mul(x).wrapping_sub(MIX_MULT_R.wrapping_mul(y));
    r ^ (r >> XSHIFT)
}

/// The words of a non-negative integer, lowest bits first (numpy `_coerce_to_uint32_array`).
fn entropy_words(seed: u64) -> Vec<u32> {
    let lo = seed as u32;
    let hi = (seed >> 32) as u32;
    if hi == 0 {
        vec![lo]
    } else {
        vec![lo, hi]
    }
}

/// `SeedSequence(seed).pool` for an integer seed, no spawn key.
fn seed_pool(seed: u64) -> [u32; POOL_SIZE] {
    let entropy = entropy_words(seed);
    let mut hash_const = INIT_A;
    let mut pool = [0u32; POOL_SIZE];
    for (i, slot) in pool.iter_mut().enumerate() {
        let e = if i < entropy.len() { entropy[i] } else { 0 };
        *slot = hashmix(e, &mut hash_const);
    }
    for i_src in 0..POOL_SIZE {
        for i_dst in 0..POOL_SIZE {
            if i_src != i_dst {
                let h = hashmix(pool[i_src], &mut hash_const);
                pool[i_dst] = mix(pool[i_dst], h);
            }
        }
    }
    for e in entropy.iter().skip(POOL_SIZE) {
        for slot in pool.iter_mut() {
            let h = hashmix(*e, &mut hash_const);
            *slot = mix(*slot, h);
        }
    }
    pool
}

/// `SeedSequence(seed).generate_state(n_words, np.uint64)`.
fn generate_state_u64(pool: &[u32; POOL_SIZE], n_words: usize) -> Vec<u64> {
    let n32 = n_words * 2;
    let mut hash_const = INIT_B;
    let mut state32 = Vec::with_capacity(n32);
    for i in 0..n32 {
        let mut d = pool[i % POOL_SIZE] ^ hash_const;
        hash_const = hash_const.wrapping_mul(MULT_B);
        d = d.wrapping_mul(hash_const);
        d ^= d >> XSHIFT;
        state32.push(d);
    }
    state32
        .chunks(2)
        .map(|c| (c[0] as u64) | ((c[1] as u64) << 32))
        .collect()
}

/// `np.random.default_rng(seed)` for an integer seed.
#[derive(Debug, Clone)]
pub struct Pcg64 {
    state: u128,
    inc: u128,
    /// The unused high half of the last 64-bit value, for `next_u32` (numpy buffers it).
    has_u32: bool,
    u32_buf: u32,
}

impl Pcg64 {
    pub fn new(seed: u64) -> Self {
        let val = generate_state_u64(&seed_pool(seed), 4);
        let initstate = ((val[0] as u128) << 64) | val[1] as u128;
        let initseq = ((val[2] as u128) << 64) | val[3] as u128;
        let mut rng = Pcg64 { state: 0, inc: (initseq << 1) | 1, has_u32: false, u32_buf: 0 };
        rng.step();
        rng.state = rng.state.wrapping_add(initstate);
        rng.step();
        rng
    }

    #[inline]
    fn step(&mut self) {
        self.state = self.state.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.step();
        let hi = (self.state >> 64) as u64;
        let lo = self.state as u64;
        (hi ^ lo).rotate_right((self.state >> 122) as u32)
    }

    /// numpy `next_uint32` of the PCG64 bit generator: the low half of a 64-bit value first,
    /// then the buffered high half.
    pub fn next_u32(&mut self) -> u32 {
        if self.has_u32 {
            self.has_u32 = false;
            return self.u32_buf;
        }
        let next = self.next_u64();
        self.has_u32 = true;
        self.u32_buf = (next >> 32) as u32;
        next as u32
    }

    /// A uniform integer in `0..=rng` (numpy `random_bounded_uint64(.., 0, rng, ..)` for `rng` below 2^32:
    /// Lemire's method on 32-bit values). `None` for a larger range (not needed here).
    pub fn bounded(&mut self, rng: u64) -> Option<u64> {
        if rng == 0 {
            return Some(0);
        }
        if rng > 0xFFFF_FFFF {
            return None;
        }
        if rng == 0xFFFF_FFFF {
            return Some(self.next_u32() as u64);
        }
        let rng_excl = rng + 1;
        let mut m = self.next_u32() as u64 * rng_excl;
        let mut leftover = m & 0xFFFF_FFFF;
        if leftover < rng_excl {
            let threshold = (0xFFFF_FFFFu64 - rng) % rng_excl;
            while leftover < threshold {
                m = self.next_u32() as u64 * rng_excl;
                leftover = m & 0xFFFF_FFFF;
            }
        }
        Some(m >> 32)
    }

    /// The set of indices that `Generator.choice(pop, size, replace=False)` picks (Floyd's algorithm), in
    /// ascending order. The numpy shuffle of the result is not done: the caller sorts the values anyway.
    /// `None` for the case that numpy handles with a tail shuffle (`pop > 10000` and `size > pop / 50`).
    pub fn choice_set(&mut self, pop: usize, size: usize) -> Option<Vec<usize>> {
        if size > pop || (pop > 10_000 && size > pop / 50) {
            return None;
        }
        let mut chosen = std::collections::BTreeSet::new();
        for j in (pop - size)..pop {
            let val = self.bounded(j as u64)? as usize;
            if !chosen.insert(val) {
                chosen.insert(j);
            }
        }
        Some(chosen.into_iter().collect())
    }

    /// A uniform in [0, 1) with 53 bits (numpy `next_double`).
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    /// `Generator.standard_normal()`: numpy ziggurat, 64-bit.
    pub fn standard_normal(&mut self) -> f64 {
        loop {
            let mut r = self.next_u64();
            let idx = (r & 0xff) as usize;
            r >>= 8;
            let sign = r & 1;
            let rabs = (r >> 1) & 0x000f_ffff_ffff_ffff;
            let mut x = rabs as f64 * WI_DOUBLE[idx];
            if sign == 1 {
                x = -x;
            }
            if rabs < KI_DOUBLE[idx] {
                return x;
            }
            if idx == 0 {
                loop {
                    let xx = -ZIGGURAT_NOR_INV_R * (-self.next_double()).ln_1p();
                    let yy = -(-self.next_double()).ln_1p();
                    if yy + yy > xx * xx {
                        return if (rabs >> 8) & 1 == 1 {
                            -(ZIGGURAT_NOR_R + xx)
                        } else {
                            ZIGGURAT_NOR_R + xx
                        };
                    }
                }
            } else if (FI_DOUBLE[idx - 1] - FI_DOUBLE[idx]) * self.next_double() + FI_DOUBLE[idx]
                < (-0.5 * x * x).exp()
            {
                return x;
            }
        }
    }

    /// `Generator.normal(mean, std)`: `mean + std * standard_normal()`.
    pub fn normal(&mut self, mean: f64, std: f64) -> f64 {
        mean + std * self.standard_normal()
    }

    /// `Generator.standard_normal(n)`.
    pub fn standard_normal_vec(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.standard_normal()).collect()
    }

    /// `Generator.standard_normal((k, n))`: row-major fill.
    pub fn standard_normal_matrix(&mut self, k: usize, n: usize) -> Vec<Vec<f64>> {
        (0..k).map(|_| self.standard_normal_vec(n)).collect()
    }

    /// `Generator.normal(mean, std, n)`.
    pub fn normal_vec(&mut self, mean: f64, std: f64, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.normal(mean, std)).collect()
    }
}
