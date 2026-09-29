//! Pruebas de integración del terreno procedural.

use diorama::material::{GRASS, SAND, WATER};
use diorama::terrain::{generate, TerrainParams};
use diorama::world::AIR;

#[test]
fn terrain_covers_at_least_16x16_columns() {
    let params = TerrainParams::default();
    assert!(params.size >= 16);
    assert_eq!(params.size, 24);
    let (_, t) = generate(&params);
    let cols: Vec<_> = t.columns().collect();
    assert!(cols.len() >= 16 * 16, "solo {} columnas", cols.len());
    let min_x = cols.iter().map(|c| c.0).min().unwrap();
    let max_x = cols.iter().map(|c| c.0).max().unwrap();
    let min_z = cols.iter().map(|c| c.1).min().unwrap();
    let max_z = cols.iter().map(|c| c.1).max().unwrap();
    assert!(max_x - min_x + 1 >= 20, "ancho en x: {}", max_x - min_x + 1);
    assert!(max_z - min_z + 1 >= 20, "ancho en z: {}", max_z - min_z + 1);
    assert!(max_x - min_x < 24 && max_z - min_z < 24);
}

#[test]
fn heights_stay_within_limits() {
    let params = TerrainParams::default();
    let (grid, t) = generate(&params);
    for (x, z, h) in t.columns() {
        assert!(
            (3..=19).contains(&h),
            "altura {h} fuera de rango en ({x}, {z})"
        );
        assert!(grid.column_top(x, z).unwrap() >= h);
        // Las columnas del interior (lejos del borde y del lago) quedan en [12, 18].
        let d = ((x as f32 + 0.5 - 16.0).powi(2) + (z as f32 + 0.5 - 16.0).powi(2)).sqrt();
        if d < 6.0 && !t.is_lake(x, z) {
            assert!((12..=18).contains(&h), "altura interior {h} en ({x}, {z})");
        }
    }
    // La isla cuelga: hay bloques muy por debajo de la superficie, cerca de y = 2.
    let lowest = grid.solid_cells().map(|(c, _)| c[1]).min().unwrap();
    assert!(lowest <= 4, "la parte inferior llega solo a y = {lowest}");
    // La superficie es grama.
    let counts = grid.material_counts();
    assert!(counts[GRASS as usize] > 300);
    assert!(counts[SAND as usize] > 5);
}

#[test]
fn lake_has_water() {
    let params = TerrainParams::default();
    let (grid, t) = generate(&params);
    let counts = grid.material_counts();
    assert!(
        counts[WATER as usize] >= 20,
        "solo {} celdas de agua",
        counts[WATER as usize]
    );
    for (c, m) in grid.solid_cells() {
        if m == WATER {
            assert!(t.is_lake(c[0], c[2]));
            assert!(c[1] < params.water_level);
        }
    }
    // El centro del lago tiene agua hasta el nivel del agua.
    let (lx, lz) = (params.lake_center.0 as i32, params.lake_center.1 as i32);
    assert_eq!(grid.get(lx, params.water_level - 1, lz), WATER);
    assert_eq!(grid.get(lx, params.water_level, lz), AIR);
}

#[test]
fn same_seed_same_world() {
    let a = generate(&TerrainParams::default());
    let b = generate(&TerrainParams::default());
    assert_eq!(a, b);
    let other = generate(&TerrainParams {
        seed: 7,
        ..TerrainParams::default()
    });
    assert_ne!(
        a.0, other.0,
        "semillas distintas deben dar terrenos distintos"
    );
}
