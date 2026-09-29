//! Grid denso de vóxeles. Cada celda guarda el id de su material (0 = aire).

use super::Aabb;
use crate::math::Vec3;

/// Id de material de una celda vacía.
pub const AIR: u8 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoxelGrid {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    cells: Vec<u8>,
}

impl VoxelGrid {
    pub fn new(nx: usize, ny: usize, nz: usize) -> VoxelGrid {
        VoxelGrid {
            nx,
            ny,
            nz,
            cells: vec![AIR; nx * ny * nz],
        }
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0
            && y >= 0
            && z >= 0
            && (x as usize) < self.nx
            && (y as usize) < self.ny
            && (z as usize) < self.nz
    }

    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (y * self.nz + z) * self.nx + x
    }

    /// Material de la celda; fuera del grid siempre es aire.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.in_bounds(x, y, z) {
            self.cells[self.index(x as usize, y as usize, z as usize)]
        } else {
            AIR
        }
    }

    #[inline]
    pub fn get_cell(&self, c: [i32; 3]) -> u8 {
        self.get(c[0], c[1], c[2])
    }

    /// Escribe una celda. Las coordenadas fuera del grid se ignoran.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, material: u8) {
        if self.in_bounds(x, y, z) {
            let i = self.index(x as usize, y as usize, z as usize);
            self.cells[i] = material;
        }
    }

    /// Rellena la caja de celdas `[min, max]` (ambos inclusive).
    pub fn fill_box(&mut self, min: [i32; 3], max: [i32; 3], material: u8) {
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                for x in min[0]..=max[0] {
                    self.set(x, y, z, material);
                }
            }
        }
    }

    /// Caja del grid completo en coordenadas de mundo (cada celda mide 1).
    pub fn bounds(&self) -> Aabb {
        Aabb::new(
            Vec3::ZERO,
            Vec3::new(self.nx as f32, self.ny as f32, self.nz as f32),
        )
    }

    /// Altura (y) de la celda sólida más alta de la columna, si existe.
    pub fn column_top(&self, x: i32, z: i32) -> Option<i32> {
        (0..self.ny as i32)
            .rev()
            .find(|&y| self.get(x, y, z) != AIR)
    }

    /// Cuántas celdas hay de cada material (índice = id).
    pub fn material_counts(&self) -> [usize; 256] {
        let mut counts = [0usize; 256];
        for &c in &self.cells {
            counts[c as usize] += 1;
        }
        counts
    }

    /// Itera sobre las celdas no vacías: `([x, y, z], material)`.
    pub fn solid_cells(&self) -> impl Iterator<Item = ([i32; 3], u8)> + '_ {
        self.cells.iter().enumerate().filter_map(move |(i, &m)| {
            if m == AIR {
                return None;
            }
            let x = i % self.nx;
            let z = (i / self.nx) % self.nz;
            let y = i / (self.nx * self.nz);
            Some(([x as i32, y as i32, z as i32], m))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_and_out_of_bounds() {
        let mut g = VoxelGrid::new(4, 3, 2);
        assert_eq!(g.get(1, 1, 1), AIR);
        g.set(1, 2, 1, 5);
        assert_eq!(g.get(1, 2, 1), 5);
        assert_eq!(g.get(-1, 0, 0), AIR);
        assert_eq!(g.get(4, 0, 0), AIR);
        g.set(10, 10, 10, 3); // se ignora sin pánico
        assert_eq!(g.column_top(1, 1), Some(2));
        assert_eq!(g.column_top(0, 0), None);
    }

    #[test]
    fn fill_box_and_counts() {
        let mut g = VoxelGrid::new(5, 5, 5);
        g.fill_box([1, 1, 1], [2, 3, 1], 7);
        let counts = g.material_counts();
        assert_eq!(counts[7], 6);
        assert_eq!(counts[AIR as usize], 125 - 6);
        let cells: Vec<_> = g.solid_cells().collect();
        assert_eq!(cells.len(), 6);
        assert!(cells.contains(&([2, 3, 1], 7)));
    }
}
