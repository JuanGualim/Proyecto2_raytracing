//! Generador pseudoaleatorio xorshift64* propio (determinista y sin dependencias).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    /// Crea el generador. La semilla 0 se reemplaza, porque xorshift no sale del estado 0.
    pub fn new(seed: u64) -> XorShift64 {
        // Mezcla la semilla (splitmix64) para que semillas parecidas den secuencias distintas.
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        XorShift64 {
            state: if z == 0 { 0x2545_f491_4f6c_dd1d } else { z },
        }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Flotante uniforme en [0, 1).
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Entero uniforme en [0, n).
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
    }

    /// Flotante uniforme en [lo, hi).
    #[inline]
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = XorShift64::new(42);
        let mut b = XorShift64::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = XorShift64::new(43);
        let mut a = XorShift64::new(42);
        let same = (0..100).filter(|_| a.next_u64() == c.next_u64()).count();
        assert_eq!(same, 0);
    }

    #[test]
    fn zero_seed_works_and_values_in_range() {
        let mut r = XorShift64::new(0);
        let mut sum = 0.0;
        for _ in 0..10_000 {
            let f = r.next_f32();
            assert!((0.0..1.0).contains(&f));
            sum += f;
            assert!(r.below(7) < 7);
            let x = r.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&x));
        }
        // Media cercana a 0.5.
        assert!((sum / 10_000.0 - 0.5).abs() < 0.02);
    }
}
