//! Ruido Perlin 2D implementado a mano y fBm (suma de octavas).

use super::rng::XorShift64;

/// Ruido Perlin 2D con tabla de permutación barajada por la semilla.
#[derive(Clone, Debug)]
pub struct Perlin {
    perm: [u8; 512],
}

/// Gradientes: 4 diagonales y 4 ejes.
const GRADS: [(f32, f32); 8] = [
    (1.0, 1.0),
    (-1.0, 1.0),
    (1.0, -1.0),
    (-1.0, -1.0),
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
];

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Perlin {
    pub fn new(seed: u64) -> Perlin {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut rng = XorShift64::new(seed);
        // Fisher–Yates.
        for i in (1..256).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            p.swap(i, j);
        }
        Perlin {
            perm: std::array::from_fn(|i| p[i & 255]),
        }
    }

    #[inline]
    fn grad(&self, ix: i32, iy: i32, dx: f32, dy: f32) -> f32 {
        let h = self.perm[(self.perm[(ix & 255) as usize] as usize + (iy & 255) as usize) & 511];
        let (gx, gy) = GRADS[(h & 7) as usize];
        gx * dx + gy * dy
    }

    /// Ruido en [-1, 1]; vale 0 en los puntos enteros de la red.
    pub fn noise2(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor();
        let y0 = y.floor();
        let (ix, iy) = (x0 as i32, y0 as i32);
        let (fx, fy) = (x - x0, y - y0);
        let n00 = self.grad(ix, iy, fx, fy);
        let n10 = self.grad(ix + 1, iy, fx - 1.0, fy);
        let n01 = self.grad(ix, iy + 1, fx, fy - 1.0);
        let n11 = self.grad(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
        let (u, v) = (fade(fx), fade(fy));
        lerp(lerp(n00, n10, u), lerp(n01, n11, u), v).clamp(-1.0, 1.0)
    }

    /// Movimiento browniano fraccional: `octaves` capas de ruido, cada una con frecuencia
    /// × `lacunarity` y amplitud × `gain`. Se normaliza para quedar en [-1, 1].
    pub fn fbm2(&self, x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for o in 0..octaves {
            // Cada octava se desplaza para que los ceros de la red no coincidan.
            let off = o as f32 * 17.31;
            sum += amp * self.noise2(x * freq + off, y * freq - off);
            norm += amp;
            amp *= gain;
            freq *= lacunarity;
        }
        if norm > 0.0 {
            sum / norm
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_noise() {
        let a = Perlin::new(7);
        let b = Perlin::new(7);
        let c = Perlin::new(8);
        let mut differ = false;
        for i in 0..200 {
            let (x, y) = (i as f32 * 0.37, i as f32 * 0.11 - 3.0);
            assert_eq!(a.noise2(x, y), b.noise2(x, y));
            assert_eq!(a.fbm2(x, y, 4, 2.0, 0.5), b.fbm2(x, y, 4, 2.0, 0.5));
            differ |= a.noise2(x, y) != c.noise2(x, y);
        }
        assert!(differ, "semillas distintas deben dar ruido distinto");
    }

    #[test]
    fn noise_in_expected_range_and_not_constant() {
        let p = Perlin::new(1);
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for i in 0..100 {
            for j in 0..100 {
                let (x, y) = (i as f32 * 0.173, j as f32 * 0.191);
                let n = p.noise2(x, y);
                let f = p.fbm2(x, y, 4, 2.0, 0.5);
                assert!((-1.0..=1.0).contains(&n));
                assert!((-1.0..=1.0).contains(&f));
                lo = lo.min(n);
                hi = hi.max(n);
            }
        }
        assert!(
            lo < -0.3 && hi > 0.3,
            "rango [{lo}, {hi}] demasiado estrecho"
        );
        // En la red entera el ruido vale 0.
        assert_eq!(p.noise2(3.0, -5.0), 0.0);
    }

    #[test]
    fn noise_is_continuous() {
        let p = Perlin::new(99);
        for i in 0..500 {
            let (x, y) = (i as f32 * 0.0931 - 20.0, i as f32 * 0.0577 + 4.0);
            let d = 1e-3;
            assert!((p.noise2(x, y) - p.noise2(x + d, y)).abs() < 0.01);
            assert!((p.noise2(x, y) - p.noise2(x, y + d)).abs() < 0.01);
            assert!((p.fbm2(x, y, 4, 2.0, 0.5) - p.fbm2(x + d, y + d, 4, 2.0, 0.5)).abs() < 0.02);
        }
    }
}
