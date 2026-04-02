use avian2d::prelude::*;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

use crate::*;

// ── Marker components for view-mode switching ─────────────────────────────────

/// Mesh visible only in map/orbit view (`MapView(true)`).
#[derive(Component)]
pub struct OrbitViewMesh;

/// Mesh visible only in terrain/close view (`MapView(false)`).
#[derive(Component)]
pub struct TerrainViewMesh;

// ── Heightmap ─────────────────────────────────────────────────────────────────

/// Number of samples around the full circle stored in the heightmap texture.
pub const HEIGHTMAP_RESOLUTION: usize = 2048;

/// Generates a heightmap by sampling the same FBM used in the orbit shader.
/// Returns `HEIGHTMAP_RESOLUTION` values in `[0, 1]`, indexed by angle.
/// Index 0 = angle −π, index N-1 = angle just below +π.
pub fn generate_heightmap(size: f32, seed: f32, octaves: i32) -> Vec<f32> {
    use std::f32::consts::{PI, TAU};
    let heights: Vec<f32> = (0..HEIGHTMAP_RESOLUTION)
        .map(|i| {
            let angle = -PI + (i as f32 / HEIGHTMAP_RESOLUTION as f32) * TAU;
            let uv = Vec2::new((angle / TAU + 0.5) * size * 20.0, 0.5);
            fbm_cpu(uv, size, seed, octaves)
        })
        .collect();

    let begin: Vec<f32> = heights[0..16].into();

    println!("{:?}", begin);

    heights
}

/// Builds a Bevy `Image` from a heightmap slice (R32Float, 1024×1).
pub fn heightmap_to_image(heights: &[f32]) -> Image {
    let data: Vec<u8> = heights.iter().flat_map(|h| h.to_le_bytes()).collect();
    let mut image = Image::new(
        Extent3d {
            width: heights.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Repeat in U so atan2 wraps cleanly at ±π.
    use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });
    image
}

/// Looks up the terrain height (world units) at `angle` using linear
/// interpolation over the pre-computed heights array.
pub fn sample_height(angle: f32, base_radius: f32, height_scale: f32, heights: &[f32]) -> f32 {
    use std::f32::consts::{PI, TAU};
    let n = heights.len() as f32;
    // Map angle (-π .. π) to index space (0 .. N).
    let t = (angle + PI) / TAU * n;
    let lo = (t.floor() as usize).rem_euclid(heights.len());
    let hi = (lo + 1).rem_euclid(heights.len());
    let frac = t - t.floor();
    let h = heights[lo] * (1.0 - frac) + heights[hi] * frac;
    base_radius + h * height_scale
}

// ── CPU FBM (used only for heightmap generation at startup) ──────────────────

fn fbm_rand(coord: Vec2, size: f32, seed: f32) -> f32 {
    let size_r = size.round();
    let mx = 2.0 * size_r;
    let my = size_r;
    let tx = coord.x - mx * (coord.x / mx).floor();
    let ty = coord.y - my * (coord.y / my).floor();
    let dot = tx * 12.9898 + ty * 78.233;
    let v = dot.sin() * 15.5453 * seed;
    v - v.floor()
}

fn fbm_noise(coord: Vec2, size: f32, seed: f32) -> f32 {
    let i = Vec2::new(coord.x.floor(), coord.y.floor());
    let f = coord - i;
    let a = fbm_rand(i, size, seed);
    let b = fbm_rand(i + Vec2::X, size, seed);
    let c = fbm_rand(i + Vec2::Y, size, seed);
    let d = fbm_rand(i + Vec2::ONE, size, seed);
    let cubic = f * f * (Vec2::splat(3.0) - 2.0 * f);
    a + (b - a) * cubic.x + (c - a) * cubic.y * (1.0 - cubic.x) + (d - b) * cubic.x * cubic.y
}

fn fbm_cpu(coord: Vec2, size: f32, seed: f32, octaves: i32) -> f32 {
    let mut value = 0.0_f32;
    let mut scale = 0.5_f32;
    let mut c = coord;
    for _ in 0..octaves {
        value += fbm_noise(c, size, seed) * scale;
        c *= 2.0;
        scale *= 0.5;
    }
    value
}

// ── Terrain component ─────────────────────────────────────────────────────────

/// Attached to each `CelestialBody` entity. Holds the pre-computed heightmap
/// array for collider generation — same data uploaded to the GPU texture.
#[derive(Component)]
pub struct BodyTerrainParams {
    pub base_radius: f32,
    pub height_scale: f32,
    pub heights: Vec<f32>,
}

// ── Terrain collider ──────────────────────────────────────────────────────────

/// Marks the dynamically-generated terrain collider entity for a celestial body.
#[derive(Component)]
pub struct TerrainColliderMarker {
    pub parent_body: Entity,
    pub last_rocket_angle: f32,
}

/// Tuning knobs for terrain collider generation.
#[derive(Resource)]
pub struct TerrainColliderConfig {
    /// Generate a terrain collider when `rocket_dist < base_radius * this`.
    pub proximity_threshold_factor: f32,
    /// Half-arc in radians covered by the polyline on each side of the rocket.
    pub arc_half_radians: f32,
    /// Number of sample points in the polyline.
    pub num_samples: usize,
    /// Rebuild the collider when the rocket moves more than this many radians.
    pub update_angle_threshold: f32,
}

impl Default for TerrainColliderConfig {
    fn default() -> Self {
        Self {
            proximity_threshold_factor: 1.15,
            arc_half_radians: std::f32::consts::FRAC_PI_2,
            num_samples: 200,
            update_angle_threshold: 0.05,
        }
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Switches mesh visibility between orbit view and terrain view when `MapView` changes.
pub fn switch_terrain_materials(
    map_view: Res<MapView>,
    mut orbit_q: Query<&mut Visibility, (With<OrbitViewMesh>, Without<TerrainViewMesh>)>,
    mut terrain_q: Query<&mut Visibility, (With<TerrainViewMesh>, Without<OrbitViewMesh>)>,
) {
    if !map_view.is_changed() {
        return;
    }
    let (orbit_vis, terrain_vis) = if map_view.0 {
        (Visibility::Visible, Visibility::Hidden)
    } else {
        (Visibility::Hidden, Visibility::Visible)
    };
    for mut v in orbit_q.iter_mut() {
        *v = orbit_vis;
    }
    for mut v in terrain_q.iter_mut() {
        *v = terrain_vis;
    }
}

fn build_terrain_polyline(
    body: &BodyTerrainParams,
    center_angle: f32,
    half_arc: f32,
    n: usize,
) -> Vec<Vec2> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            let angle = center_angle - half_arc + t * 2.0 * half_arc;
            let r = sample_height(angle, body.base_radius, body.height_scale, &body.heights);
            Vec2::new(r * angle.cos(), r * angle.sin())
        })
        .collect()
}

/// Generates (or updates) a terrain polyline collider near the rocket when close
/// to a celestial body's surface.
pub fn update_terrain_colliders(
    map_view: Res<MapView>,
    rocket_q: Query<&Transform, With<Rocket>>,
    body_q: Query<(Entity, &Transform, &BodyTerrainParams)>,
    collider_q: Query<(Entity, &TerrainColliderMarker)>,
    config: Res<TerrainColliderConfig>,
    mut commands: Commands,
) {
    if map_view.0 {
        for (e, _) in collider_q.iter() {
            commands.entity(e).despawn();
        }
        return;
    }

    let Ok(rocket_tf) = rocket_q.get_single() else {
        return;
    };
    let rocket_pos = rocket_tf.translation.truncate();

    for (body_entity, body_tf, terrain) in body_q.iter() {
        let body_pos = body_tf.translation.truncate();
        let to_rocket = rocket_pos - body_pos;
        let dist = to_rocket.length();
        let threshold = terrain.base_radius * config.proximity_threshold_factor;

        let existing = collider_q
            .iter()
            .find(|(_, m)| m.parent_body == body_entity);

        if dist > threshold {
            if let Some((e, _)) = existing {
                commands.entity(e).despawn();
            }
            continue;
        }

        let rocket_angle = to_rocket.y.atan2(to_rocket.x);

        let needs_rebuild = match existing {
            None => true,
            Some((_, m)) => {
                (rocket_angle - m.last_rocket_angle).abs() > config.update_angle_threshold
            }
        };

        if !needs_rebuild {
            continue;
        }

        if let Some((e, _)) = existing {
            commands.entity(e).despawn();
        }

        let points = build_terrain_polyline(terrain, rocket_angle, config.arc_half_radians, config.num_samples);

        commands.spawn((
            TerrainColliderMarker {
                parent_body: body_entity,
                last_rocket_angle: rocket_angle,
            },
            RigidBody::Static,
            Collider::polyline(points, None),
            Transform::from_translation(body_pos.extend(0.0)),
        ));
    }
}

/// Keeps terrain collider entities co-located with their parent body (important
/// for the moving moon).
pub fn sync_terrain_collider_to_body(
    body_q: Query<(Entity, &Transform), With<CelestialBody>>,
    mut collider_q: Query<(&TerrainColliderMarker, &mut Transform), Without<CelestialBody>>,
) {
    for (marker, mut tf) in collider_q.iter_mut() {
        if let Ok((_, body_tf)) = body_q.get(marker.parent_body) {
            tf.translation = body_tf.translation;
        }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainColliderConfig>()
            .add_systems(
                Update,
                switch_terrain_materials.run_if(in_state(AppState::Playing)),
            );
            // .add_systems(
            //     FixedUpdate,
            //     (update_terrain_colliders, sync_terrain_collider_to_body)
            //         .chain()
            //         .after(update_bodies)
            //         .run_if(in_state(AppState::Playing)),
            // );
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_unit_test_placeholder() {
        
        let planet_seed = 1234.0;
        let moon_seed = 1234.0;

        // Generate heightmaps on CPU — same data goes to GPU texture and collider.
        // Low octave count → broad peaks/valleys (lunar lander feel, not fractal noise)
        let _planet_heights = generate_heightmap(7.315, planet_seed, 2);
        let _moon_heights = generate_heightmap(8.0, moon_seed, 4);
    }
}

