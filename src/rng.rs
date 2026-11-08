//! Tiny xorshift RNG. Avoids pulling in `rand`/`getrandom`, which need extra setup on wasm.

use bevy::prelude::*;

#[derive(Resource)]
pub struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        Rng(0x9E37_79B9_7F4A_7C15)
    }
}

impl Rng {
    /// Mix in some entropy, e.g. the moment players pressed start.
    pub fn reseed(&mut self, entropy: u64) {
        self.0 ^= entropy.wrapping_mul(0x2545_F491_4F6C_DD1D);
        if self.0 == 0 {
            self.0 = 1;
        }
        for _ in 0..4 {
            self.next_u64();
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }

    pub fn index(&mut self, len: usize) -> usize {
        (self.next_u64() % len as u64) as usize
    }

    pub fn angle(&mut self) -> f32 {
        self.range(0.0, std::f32::consts::TAU)
    }
}
