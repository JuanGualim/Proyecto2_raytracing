//! Geometría del mundo: grid de vóxeles, cajas y recorrido DDA.

pub mod aabb;
pub mod dda;
pub mod voxel_grid;

pub use aabb::Aabb;
pub use dda::Hit;
pub use voxel_grid::{VoxelGrid, AIR};
