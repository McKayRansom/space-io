// Rust doesn't like the Uniforms, probably because they appear never to be read (they are read in shader code0)
#![allow(unused)]

use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
    sprite::{AlphaMode2d, Material2d, Material2dPlugin},
};
use rand::Rng;

#[derive(Resource)]
pub struct Materials {
    planet: Assets<PlanetMaterial>,
    cloud: Assets<CloudMaterial>,
    moon_surface: Assets<MoonSurfaceMaterial>,
    moon_crater: Assets<MoonCraterMaterial>,
}

// ── Planet shader material ────────────────────────────────────────────────────
// Layout must match planet.wgsl PlanetMaterialUniform (std140, 160 bytes total):
//   colors[6]: 96 bytes, light_origin: 8, then 13 scalars × 4 = 52, _pad0/_pad1 × 4 = 8

#[derive(ShaderType, Clone, Debug)]
struct PlanetMaterialUniform {
    colors: [Vec4; 6],
    light_origin: Vec2,
    pixels: f32,
    rotation: f32,
    time_speed: f32,
    dither_size: f32,
    light_border_1: f32,
    light_border_2: f32,
    river_cutoff: f32,
    size: f32,
    seed: f32,
    octaves: i32,
    time: f32,
    should_dither: u32,
    _pad0: u32,
    _pad1: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PlanetMaterial {
    #[uniform(0)]
    params: PlanetMaterialUniform,
}

impl Material2d for PlanetMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/planet.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
// ── Planet ─────────────────────────────────────────────────────────────
// Colors from Rivers.tscn default scheme:
//   [0] bright green, [1] mid green, [2] dark teal, [3] shadow blue-grey
//   [4] shallow river blue, [5] deep river blue
pub fn planet_mat(materials: &mut Assets<PlanetMaterial>) -> Handle<PlanetMaterial> {
    materials.add(PlanetMaterial {
        params: PlanetMaterialUniform {
            colors: [
                planet_color(0.388, 0.671, 0.247), // bright green (lit land)
                planet_color(0.231, 0.490, 0.310), // mid green
                planet_color(0.184, 0.341, 0.325), // dark teal
                planet_color(0.157, 0.208, 0.251), // shadow blue-grey
                planet_color(0.310, 0.643, 0.722), // shallow river
                planet_color(0.251, 0.286, 0.451), // deep river
            ],
            light_origin: Vec2::new(0.39, 0.39),
            pixels: 100.0,
            rotation: 0.2,
            time_speed: 0.1,
            dither_size: 3.951,
            light_border_1: 0.287,
            light_border_2: 0.476,
            river_cutoff: 0.368,
            size: 4.6,
            seed: rand::thread_rng().gen_range(1.0f32..10.0f32),
            octaves: 6,
            time: 0.0,
            should_dither: 1,
            _pad0: 0,
            _pad1: 0,
        },
    })
}

// ── Cloud shader material ─────────────────────────────────────────────────────
// Layout must match cloud.wgsl CloudMaterialUniform (std140, 128 bytes total):
//   colors[4]: 64 bytes, light_origin: 8, then 10 scalars × 4 = 40, _pad0/_pad1 × 4 = 8

#[derive(ShaderType, Clone, Debug)]
struct CloudMaterialUniform {
    colors: [Vec4; 4],
    light_origin: Vec2,
    pixels: f32,
    rotation: f32,
    cloud_cover: f32,
    time_speed: f32,
    stretch: f32,
    cloud_curve: f32,
    light_border_1: f32,
    light_border_2: f32,
    size: f32,
    seed: f32,
    octaves: i32,
    time: f32,
    _pad0: u32,
    _pad1: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CloudMaterial {
    #[uniform(0)]
    params: CloudMaterialUniform,
}

impl Material2d for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cloud.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// ── Cloud layer ────────────────────────────────────────────────────────
// Colors from Rivers.tscn cloud shader (id="2"), sRGB→linear converted:
//   [0] bright white, [1] off-white, [2] mid-shadow, [3] dark shadow
pub fn cloud_mat(materials: &mut Assets<CloudMaterial>) -> Handle<CloudMaterial> {
    materials.add(CloudMaterial {
        params: CloudMaterialUniform {
            colors: [
                planet_color(0.961, 1.000, 0.910), // bright cloud (lit)
                planet_color(0.875, 0.878, 0.910), // off-white cloud
                planet_color(0.408, 0.435, 0.600), // mid shadow
                planet_color(0.251, 0.286, 0.451), // dark shadow
            ],
            light_origin: Vec2::new(0.39, 0.39),
            pixels: 100.0,
            rotation: 0.0,
            cloud_cover: 0.47,
            time_speed: 0.1,
            stretch: 2.0,
            cloud_curve: 1.3,
            light_border_1: 0.52,
            light_border_2: 0.62,
            size: 7.315,
            seed: rand::thread_rng().gen_range(1.0f32..10.0f32),
            octaves: 2,
            time: 0.0,
            _pad0: 0,
            _pad1: 0,
        },
    })
}

// ── Moon shader materials ─────────────────────────────────────────────────────
// MoonSurfaceUniform: colors[3]=48, light_origin=8, 11 scalars=44, 3 pads=12 → 112 bytes (7×16 ✓)
// MoonCraterUniform:  colors[2]=32, light_origin=8,  7 scalars=28, 3 pads=12 →  80 bytes (5×16 ✓)

#[derive(ShaderType, Clone, Debug)]
struct MoonSurfaceUniform {
    colors: [Vec4; 3],
    light_origin: Vec2,
    pixels: f32,
    rotation: f32,
    time_speed: f32,
    dither_size: f32,
    light_border_1: f32,
    light_border_2: f32,
    size: f32,
    seed: f32,
    octaves: i32,
    time: f32,
    should_dither: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct MoonSurfaceMaterial {
    #[uniform(0)]
    params: MoonSurfaceUniform,
}

impl Material2d for MoonSurfaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/moon_surface.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(ShaderType, Clone, Debug)]
struct MoonCraterUniform {
    colors: [Vec4; 2],
    light_origin: Vec2,
    pixels: f32,
    rotation: f32,
    time_speed: f32,
    light_border: f32,
    size: f32,
    seed: f32,
    time: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct MoonCraterMaterial {
    #[uniform(0)]
    params: MoonCraterUniform,
}

impl Material2d for MoonCraterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/moon_craters.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
// ── Moon ───────────────────────────────────────────────────────────────
// Colors from NoAtmosphere.tscn (sRGB→linear):
//   Surface [0]=light grey-blue, [1]=mid blue-grey, [2]=dark blue-grey
//   Craters [0]=mid blue-grey (lit), [1]=dark blue-grey (shadow)
#[rustfmt::skip]
fn moon_surface_mat(assets: &mut Assets<MoonSurfaceMaterial>, seed: f32) -> Handle<MoonSurfaceMaterial> {
    assets.add(MoonSurfaceMaterial {
        params: MoonSurfaceUniform {
            colors: [
                planet_color(0.639, 0.655, 0.761), // light grey-blue (lit)
                planet_color(0.298, 0.408, 0.522), // mid blue-grey
                planet_color(0.227, 0.247, 0.369), // dark blue-grey (shadow)
            ],
            light_origin:   Vec2::new(0.25, 0.25),
            pixels:         100.0,
            rotation:       0.0,
            time_speed:     0.4,
            dither_size:    2.0,
            light_border_1: 0.615,
            light_border_2: 0.729,
            size:           8.0,
            seed,
            octaves:        4,
            time:           0.0,
            should_dither:  1,
            _pad0:          0,
            _pad1:          0,
            _pad2:          0,
        },
    })
}

#[rustfmt::skip]
fn moon_crater_mat(assets: &mut Assets<MoonCraterMaterial>, seed: f32) -> Handle<MoonCraterMaterial> {
    assets.add(MoonCraterMaterial {
        params: MoonCraterUniform {
            colors: [
                planet_color(0.298, 0.408, 0.522), // mid blue-grey (crater lit)
                planet_color(0.227, 0.247, 0.369), // dark blue-grey (crater shadow)
            ],
            light_origin:   Vec2::new(0.25, 0.25),
            pixels:         87.419,
            rotation:       0.0,
            time_speed:     0.001,
            light_border:   0.465,
            size:           5.0,
            seed,
            time:           0.0,
            _pad0:          0,
            _pad1:          0,
            _pad2:          0,
        },
    })
}

/// Convert an sRGB channel value to linear light (matches Godot's source_color hint behaviour).
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055_f32).powf(2.4)
    }
}

/// Build a Vec4 from sRGB display values, converting to linear for the shader.
fn planet_color(r: f32, g: f32, b: f32) -> Vec4 {
    Vec4::new(srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0)
}

fn animate_planet_time(
    time: Res<Time>,
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    mut cloud_materials: ResMut<Assets<CloudMaterial>>,
    mut moon_surface_materials: ResMut<Assets<MoonSurfaceMaterial>>,
    mut moon_crater_materials: ResMut<Assets<MoonCraterMaterial>>,
) {
    // slow-down time slightly
    let dt = time.delta_secs() * 0.1;

    for (_, mat) in planet_materials.iter_mut() {
        mat.params.time += dt;
    }
    for (_, mat) in cloud_materials.iter_mut() {
        mat.params.time += dt;
    }
    for (_, mat) in moon_surface_materials.iter_mut() {
        mat.params.time += dt;
    }
    for (_, mat) in moon_crater_materials.iter_mut() {
        mat.params.time += dt;
    }
}

/// Handles to all custom shader materials, created at startup via [`FromWorld`].
/// Add new material handles here instead of threading `ResMut<Assets<XxxMaterial>>`
/// through `setup` in `main.rs`.
#[derive(Resource)]
pub struct CelestialMaterials {
    pub planet: Handle<PlanetMaterial>,
    pub cloud: Handle<CloudMaterial>,
    pub moon_surface: Handle<MoonSurfaceMaterial>,
    pub moon_crater: Handle<MoonCraterMaterial>,
}

impl FromWorld for CelestialMaterials {
    fn from_world(world: &mut World) -> Self {
        let planet = planet_mat(&mut world.resource_mut::<Assets<PlanetMaterial>>());
        let cloud = cloud_mat(&mut world.resource_mut::<Assets<CloudMaterial>>());
        // Share the same seed between surface and crater layers.
        let moon_seed = rand::thread_rng().gen_range(1.0f32..10.0f32);
        let moon_surface = moon_surface_mat(&mut world.resource_mut::<Assets<MoonSurfaceMaterial>>(), moon_seed);
        let moon_crater = moon_crater_mat(&mut world.resource_mut::<Assets<MoonCraterMaterial>>(), moon_seed);
        Self { planet, cloud, moon_surface, moon_crater }
    }
}

pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<PlanetMaterial>::default(),
            Material2dPlugin::<CloudMaterial>::default(),
            Material2dPlugin::<MoonSurfaceMaterial>::default(),
            Material2dPlugin::<MoonCraterMaterial>::default(),
        ))
        // Material2dPlugins above have already registered Assets<T>, so FromWorld
        // can safely create handles immediately.
        .init_resource::<CelestialMaterials>()
        .add_systems(Update, animate_planet_time);
    }
}
