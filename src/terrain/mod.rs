//! Terreno procedural: PRNG, ruido Perlin/fBm y generador de la isla flotante.

pub mod generator;
pub mod noise;
pub mod rng;

pub use generator::{generate, Terrain, TerrainParams};
