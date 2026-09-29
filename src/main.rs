//! Punto de entrada: parseo de argumentos, presets y orquestación del render.

use diorama::camera::Camera;
use diorama::image_io;
use diorama::math::{Ray, Vec3};
use diorama::parallel::{self, FrameStats};
use diorama::renderer::RenderSettings;
use diorama::scene::{Scene, SceneConfig};
use diorama::texture_gen::AssetSource;
use diorama::world::dda;
use std::path::{Path, PathBuf};
use std::time::Instant;

const HELP: &str = "\
diorama — raytracer por CPU de una isla flotante al atardecer

USO:
    diorama [opciones]

OPCIONES:
    --preset preview|final   preview = 640x360, 1 spp, profundidad 3
                             final   = 1280x720, 4 spp, profundidad 4
    --width N  --height N    Resolución (sobrescribe el preset)
    --spp N                  Muestras por píxel
    --depth N                Profundidad máxima de recursión
    --still                  Un solo frame en out/still.bmp (y out/still.ppm)
    --frames N               Número de frames de la animación (288 por defecto)
    --start K                Reanudar la animación desde el frame K
    --seed S                 Semilla del terreno
    --threads N              Número de hilos (por defecto, todos los disponibles)
    --yaw G  --pitch G       Ángulos de la cámara en grados (modo still)
    --dist D                 Distancia de la cámara al objetivo (modo still)
    --focus F                Objetivo de la cámara en modo still:
                             island | statue | greenhouse | ruins | lake
    --bench                  Mide ms/frame con 1, 2, 4 y N hilos, y DDA vs fuerza bruta
    --help                   Muestra esta ayuda
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preset {
    Preview,
    Final,
}

impl Preset {
    fn settings(self) -> RenderSettings {
        match self {
            Preset::Preview => RenderSettings {
                width: 640,
                height: 360,
                spp: 1,
                max_depth: 3,
                exposure: 1.0,
            },
            Preset::Final => RenderSettings {
                width: 1280,
                height: 720,
                spp: 4,
                max_depth: 4,
                exposure: 1.0,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Options {
    settings: RenderSettings,
    still: bool,
    frames: u32,
    start: u32,
    seed: u64,
    threads: usize,
    yaw: Option<f32>,
    pitch: Option<f32>,
    dist: Option<f32>,
    focus: String,
    bench: bool,
    help: bool,
}

fn parse_value<T: std::str::FromStr>(flag: &str, value: Option<String>) -> Result<T, String> {
    let v = value.ok_or_else(|| format!("falta el valor de {flag}"))?;
    v.parse()
        .map_err(|_| format!("valor inválido para {flag}: '{v}'"))
}

/// Parsea los argumentos (sin el nombre del programa).
fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut preset = Preset::Preview;
    let (mut width, mut height, mut spp, mut depth) = (None, None, None, None);
    let mut opts = Options {
        settings: preset.settings(),
        still: false,
        frames: 288,
        start: 0,
        seed: 2024,
        threads: parallel::available_threads(),
        yaw: None,
        pitch: None,
        dist: None,
        focus: "island".into(),
        bench: false,
        help: false,
    };
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--preset" => {
                let v: String = parse_value(&flag, it.next())?;
                preset = match v.as_str() {
                    "preview" => Preset::Preview,
                    "final" => Preset::Final,
                    _ => return Err(format!("preset desconocido: '{v}' (preview|final)")),
                };
            }
            "--width" => width = Some(parse_value::<usize>(&flag, it.next())?),
            "--height" => height = Some(parse_value::<usize>(&flag, it.next())?),
            "--spp" => spp = Some(parse_value::<u32>(&flag, it.next())?),
            "--depth" => depth = Some(parse_value::<u32>(&flag, it.next())?),
            "--still" => opts.still = true,
            "--frames" => opts.frames = parse_value(&flag, it.next())?,
            "--start" => opts.start = parse_value(&flag, it.next())?,
            "--seed" => opts.seed = parse_value(&flag, it.next())?,
            "--threads" => opts.threads = parse_value(&flag, it.next())?,
            "--yaw" => opts.yaw = Some(parse_value(&flag, it.next())?),
            "--pitch" => opts.pitch = Some(parse_value(&flag, it.next())?),
            "--dist" => opts.dist = Some(parse_value(&flag, it.next())?),
            "--focus" => {
                let f: String = parse_value(&flag, it.next())?;
                if !["island", "statue", "greenhouse", "ruins", "lake"].contains(&f.as_str()) {
                    return Err(format!("objetivo desconocido: '{f}'"));
                }
                opts.focus = f;
            }
            "--bench" => opts.bench = true,
            "--help" | "-h" => opts.help = true,
            other => return Err(format!("opción desconocida: '{other}' (usa --help)")),
        }
    }
    let mut s = preset.settings();
    s.width = width.unwrap_or(s.width);
    s.height = height.unwrap_or(s.height);
    s.spp = spp.unwrap_or(s.spp);
    s.max_depth = depth.unwrap_or(s.max_depth);
    if s.width == 0 || s.height == 0 || s.spp == 0 {
        return Err("ancho, alto y spp deben ser mayores que 0".into());
    }
    if opts.threads == 0 {
        return Err("--threads debe ser mayor que 0".into());
    }
    if opts.frames == 0 {
        return Err("--frames debe ser mayor que 0".into());
    }
    opts.settings = s;
    Ok(opts)
}

fn build_scene(opts: &Options) -> std::io::Result<Scene> {
    Scene::island(&SceneConfig {
        seed: opts.seed,
        assets: AssetSource::Directory(PathBuf::from("assets")),
        sky_size: 256,
    })
}

fn default_camera(scene: &Scene) -> Camera {
    Camera::new(
        scene.landmarks.island_center,
        (-125f32).to_radians(),
        24f32.to_radians(),
        38.0,
    )
}

fn still_camera(scene: &Scene, opts: &Options) -> Camera {
    let mut cam = default_camera(scene);
    let l = &scene.landmarks;
    cam.target = match opts.focus.as_str() {
        "statue" => l.statue,
        "greenhouse" => l.greenhouse,
        "ruins" => l.ruins,
        "lake" => l.lake,
        _ => l.island_center,
    };
    if let Some(y) = opts.yaw {
        cam.yaw = y.to_radians();
    }
    if let Some(p) = opts.pitch {
        cam.set_pitch(p.to_radians());
    }
    if let Some(d) = opts.dist {
        cam.set_distance(d);
    }
    cam
}

fn report(label: &str, s: &RenderSettings, stats: &FrameStats) {
    println!(
        "{label}: {}x{} {} spp, {:.1} ms, {:.2} Mrayos/s, {} hilos",
        s.width,
        s.height,
        s.spp,
        stats.millis(),
        stats.rays_per_second() / 1e6,
        stats.threads
    );
}

fn run_still(scene: &Scene, opts: &Options) -> std::io::Result<()> {
    let cam = still_camera(scene, opts);
    let (img, stats) = parallel::render_frame(scene, &cam, &opts.settings, 0, opts.threads);
    image_io::save(Path::new("out/still.bmp"), &img)?;
    image_io::save(Path::new("out/still.ppm"), &img)?;
    report("out/still.bmp", &opts.settings, &stats);
    Ok(())
}

fn run_animation(scene: &Scene, opts: &Options) -> std::io::Result<()> {
    let dir = Path::new("out/frames");
    std::fs::create_dir_all(dir)?;
    let total = Instant::now();
    let mut rendered = 0u32;
    for f in opts.start..opts.frames {
        let mut cam = default_camera(scene);
        cam.yaw += std::f32::consts::TAU * f as f32 / opts.frames as f32;
        let (img, stats) = parallel::render_frame(scene, &cam, &opts.settings, f, opts.threads);
        let path = dir.join(format!("frame_{f:04}.bmp"));
        image_io::save(&path, &img)?;
        report(
            &format!("frame {f:4}/{}", opts.frames),
            &opts.settings,
            &stats,
        );
        rendered += 1;
    }
    println!(
        "{rendered} frames en {:.1} s",
        total.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Mide ms/frame con distintos números de hilos y compara DDA contra fuerza bruta.
fn run_bench(scene: &Scene, opts: &Options) {
    let cam = still_camera(scene, opts);
    let n = parallel::available_threads();
    let mut counts = vec![1, 2, 4, n];
    counts.sort_unstable();
    counts.dedup();
    let s = &opts.settings;
    println!(
        "### Hilos ({}x{}, {} spp, profundidad {})\n",
        s.width, s.height, s.spp, s.max_depth
    );
    println!("| Hilos | ms/frame | Mrayos/s | Aceleración |");
    println!("|---:|---:|---:|---:|");
    let mut base = None;
    for &t in &counts {
        // Se toma el mejor de 3 corridas para reducir el ruido.
        let best = (0..3)
            .map(|_| parallel::render_frame(scene, &cam, s, 0, t).1)
            .min_by(|a, b| a.elapsed.cmp(&b.elapsed))
            .expect("al menos una corrida");
        let ms = best.millis();
        let base_ms = *base.get_or_insert(ms);
        println!(
            "| {t} | {ms:.1} | {:.2} | {:.2}× |",
            best.rays_per_second() / 1e6,
            base_ms / ms
        );
    }

    // DDA vs fuerza bruta: solo rayos primarios, escena de demostración pequeña.
    let small = Scene::demo(&AssetSource::Generated).expect("escena de demostración");
    let (w, h) = (320, 180);
    let frame = Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.45, 19.0).frame(w, h);
    let rays: Vec<Ray> = (0..w * h)
        .map(|i| frame.ray((i % w) as f32 + 0.5, (i / w) as f32 + 0.5))
        .collect();
    let cubes = small.grid.solid_cells().count();
    let t0 = Instant::now();
    let dda_hits = rays
        .iter()
        .filter(|r| dda::trace(&small.grid, r, 0.0, f32::INFINITY).is_some())
        .count();
    let dda_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t0 = Instant::now();
    let brute_hits = rays
        .iter()
        .filter(|r| dda::brute_force_trace(&small.grid, r, 0.0, f32::INFINITY).is_some())
        .count();
    let brute_ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!("\n### DDA vs fuerza bruta ({w}x{h} rayos primarios, {cubes} cubos, 1 hilo)\n");
    println!("| Método | ms | Hits | Aceleración |");
    println!("|---|---:|---:|---:|");
    println!("| Fuerza bruta (todos los cubos) | {brute_ms:.1} | {brute_hits} | 1.00× |");
    println!(
        "| DDA (Amanatides & Woo) | {dda_ms:.1} | {dda_hits} | {:.1}× |",
        brute_ms / dda_ms
    );
}

fn main() {
    let opts = match parse_args(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}\n\n{HELP}");
            std::process::exit(2);
        }
    };
    if opts.help {
        print!("{HELP}");
        return;
    }
    let scene = match build_scene(&opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error al construir la escena: {e}");
            std::process::exit(1);
        }
    };
    let result = if opts.bench {
        run_bench(&scene, &opts);
        Ok(())
    } else if opts.still {
        run_still(&scene, &opts)
    } else {
        run_animation(&scene, &opts)
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn presets() {
        let o = parse_args(args("")).unwrap();
        assert_eq!(o.settings, Preset::Preview.settings());
        assert_eq!(
            (o.settings.width, o.settings.height, o.settings.spp),
            (640, 360, 1)
        );
        let o = parse_args(args("--preset final")).unwrap();
        assert_eq!(
            (o.settings.width, o.settings.height, o.settings.spp),
            (1280, 720, 4)
        );
        assert_eq!(o.settings.max_depth, 4);
        assert_eq!(o.frames, 288);
    }

    #[test]
    fn overrides_win_over_preset_in_any_order() {
        let o = parse_args(args("--spp 9 --preset final --width 320 --depth 2")).unwrap();
        assert_eq!(o.settings.spp, 9);
        assert_eq!(o.settings.width, 320);
        assert_eq!(o.settings.height, 720);
        assert_eq!(o.settings.max_depth, 2);
    }

    #[test]
    fn other_flags() {
        let o = parse_args(args(
            "--still --frames 10 --start 3 --seed 7 --threads 2 --yaw 45 --pitch 20 --dist 30",
        ))
        .unwrap();
        assert!(o.still);
        assert_eq!((o.frames, o.start, o.seed, o.threads), (10, 3, 7, 2));
        assert_eq!(
            (o.yaw, o.pitch, o.dist),
            (Some(45.0), Some(20.0), Some(30.0))
        );
        assert!(parse_args(args("--bench")).unwrap().bench);
        assert_eq!(parse_args(args("--focus statue")).unwrap().focus, "statue");
        assert!(parse_args(args("--focus luna")).is_err());
        assert!(parse_args(args("--help")).unwrap().help);
    }

    #[test]
    fn errors() {
        assert!(parse_args(args("--preset ultra")).is_err());
        assert!(parse_args(args("--width")).is_err());
        assert!(parse_args(args("--width abc")).is_err());
        assert!(parse_args(args("--threads 0")).is_err());
        assert!(parse_args(args("--spp 0")).is_err());
        assert!(parse_args(args("--nada")).is_err());
    }
}
