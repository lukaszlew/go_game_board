// Park-Miller "minimal standard" PRNG - must match C++ implementation exactly
pub struct FastRandom {
    seed: u32,
}

impl FastRandom {
    pub fn new(seed: u32) -> Self {
        FastRandom { seed }
    }

    pub fn get_next_uint(&mut self) -> u32 {
        let lo = 16807u32.wrapping_mul(self.seed & 0xffff);
        let hi = 16807u32.wrapping_mul(self.seed >> 16);
        let lo = lo.wrapping_add((hi & 0x7fff) << 16);
        let lo = lo.wrapping_add(hi >> 15);
        self.seed = (lo & 0x7FFFFFFF).wrapping_add(lo >> 31);
        self.seed
    }

    pub fn next_double(&mut self, scale: f64) -> f64 {
        const INV_MAX_UINT: f64 = 1.0 / ((1u64 << 31) as f64);
        let s = self.get_next_uint();
        (s as f64) * (INV_MAX_UINT * scale)
    }
}
