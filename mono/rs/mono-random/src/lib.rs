//! Grey Hack Mono-compatible `System.Random`; identical seeds produce identical sequences.

use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

const MBIG: i32 = i32::MAX;
const MSEED: i32 = 161_803_398;
const SIZE: usize = 56;
const FIRST_INEXTP: usize = 21;

#[derive(Debug, Clone)]
pub struct Random {
    seeds: [i32; SIZE],
    inext: usize,
    inextp: usize,
}

impl Random {
    #[must_use]
    pub fn new(seed: i32) -> Self {
        let subtraction = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut seeds = [0; SIZE];
        let mut mj = MSEED - subtraction;

        seeds[SIZE - 1] = mj;

        let mut mk = 1;

        for i in 1..SIZE - 1 {
            let ii = (21 * i) % (SIZE - 1);

            seeds[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(MBIG);
            }

            mj = seeds[ii];
        }

        for _ in 1..5 {
            for i in 1..SIZE {
                seeds[i] = seeds[i].wrapping_sub(seeds[1 + (i + 30) % (SIZE - 1)]);
                if seeds[i] < 0 {
                    seeds[i] = seeds[i].wrapping_add(MBIG);
                }
            }
        }

        Self {
            seeds,
            inext: 0,
            inextp: FIRST_INEXTP,
        }
    }

    #[must_use]
    pub fn from_entropy() -> Self {
        #[allow(clippy::cast_possible_truncation)]
        let seed = RandomState::new().hash_one(0u8) as i32;

        Self::new(seed)
    }

    fn sample(&mut self) -> i32 {
        self.inext = if self.inext + 1 >= SIZE {
            1
        } else {
            self.inext + 1
        };
        self.inextp = if self.inextp + 1 >= SIZE {
            1
        } else {
            self.inextp + 1
        };

        let mut value = self.seeds[self.inext].wrapping_sub(self.seeds[self.inextp]);

        if value == MBIG {
            value -= 1;
        }

        if value < 0 {
            value = value.wrapping_add(MBIG);
        }

        self.seeds[self.inext] = value;

        value
    }

    /// Returns the next raw `System.Random.Next()` value.
    pub fn next_int(&mut self) -> i32 {
        self.sample()
    }

    pub fn next_double(&mut self) -> f64 {
        f64::from(self.sample()) * (1.0 / f64::from(MBIG))
    }

    pub fn next_below(&mut self, max: usize) -> usize {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let index = (self.next_double() * max as f64) as usize;

        index
    }
}
