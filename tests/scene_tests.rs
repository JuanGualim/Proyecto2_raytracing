//! Pruebas de integración de la escena completa.

use diorama::material::{GLOWSTONE, GOLD, MAIN_MATERIALS, STONE, WATER};
use diorama::scene::{Scene, SceneConfig};

fn island() -> Scene {
    Scene::island(&SceneConfig {
        sky_size: 32,
        ..SceneConfig::default()
    })
    .unwrap()
}

#[test]
fn scene_contains_the_seven_main_materials() {
    let scene = island();
    let counts = scene.grid.material_counts();
    for m in MAIN_MATERIALS {
        assert!(
            counts[m as usize] > 0,
            "falta el material {}",
            scene.material(m).name
        );
    }
}

#[test]
fn at_least_four_emissive_lights() {
    let scene = island();
    let glow = scene.grid.material_counts()[GLOWSTONE as usize];
    assert!(scene.lights.points.len() >= 4);
    assert_eq!(scene.lights.points.len(), glow);
    for l in &scene.lights.points {
        assert_eq!(scene.grid.get_cell(l.cell), GLOWSTONE);
    }
}

#[test]
fn golden_statue_stands_next_to_the_water() {
    let scene = island();
    let gold: Vec<[i32; 3]> = scene
        .grid
        .solid_cells()
        .filter(|&(_, m)| m == GOLD)
        .map(|(c, _)| c)
        .collect();
    let water: Vec<[i32; 3]> = scene
        .grid
        .solid_cells()
        .filter(|&(_, m)| m == WATER)
        .map(|(c, _)| c)
        .collect();
    assert!(gold.len() >= 15, "la estatua tiene {} bloques", gold.len());
    // Distancia horizontal mínima entre la estatua y el agua.
    let min_d2 = gold
        .iter()
        .flat_map(|g| {
            water
                .iter()
                .map(move |w| (g[0] - w[0]).pow(2) + (g[2] - w[2]).pow(2))
        })
        .min()
        .unwrap();
    assert!(min_d2 <= 9, "la estatua está a √{min_d2} bloques del agua");
    // Se apoya en un pedestal de piedra.
    let lowest = gold.iter().map(|g| g[1]).min().unwrap();
    for g in gold.iter().filter(|g| g[1] == lowest) {
        assert_eq!(scene.grid.get(g[0], g[1] - 1, g[2]), STONE);
    }
    // El punto de interés de la estatua está cerca de sus bloques.
    let c = scene.landmarks.statue;
    assert!(gold
        .iter()
        .any(|g| (g[0] as f32 + 0.5 - c.x).abs() < 2.0 && (g[2] as f32 + 0.5 - c.z).abs() < 2.0));
}

#[test]
fn scene_has_fog_and_sky_based_ambient() {
    let scene = island();
    assert!(scene.fog_density > 0.0);
    assert!(scene.lights.ambient.max_component() > 0.0);
    // La luz ambiental se deriva del promedio del cielo.
    let a = scene.lights.ambient;
    let s = scene.skybox.average;
    assert!((a.x / s.x - a.z / s.z).abs() < 1e-4);
}

#[test]
fn island_is_a_few_thousand_cubes() {
    let n = island().grid.solid_cells().count();
    println!("cubos en la isla: {n}");
    assert!(n > 2000, "la isla tiene solo {n} cubos");
}
