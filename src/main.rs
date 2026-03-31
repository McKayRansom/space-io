//! Space IO – 2D KSP-style prototype (Bevy 0.15)
//!
//! Controls
//! --------
//!   A / ←      rotate left  (CCW)
//!   D / →      rotate right (CW)
//!   Space/↑/W  main engine  (thrust)
//!   R          reset to orbit

use std::f32::consts::FRAC_PI_2;

use avian2d::{prelude::*, sync::ancestor_marker::AncestorMarker};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
    sprite::{Material2d, Material2dPlugin},
};
use rand::Rng;

// ── Physics constants  ─────────────────────────
const G: f32 = 20.0;
const PLANET_MASS: f32 = 5.0e6; // G·M = 2·10⁷
const PLANET_RADIUS: f32 = 3400.0;
const PLANET_SURFACE_GRAVITY: f32 = G * PLANET_MASS / (PLANET_RADIUS * PLANET_RADIUS);
const FUEL_RATE: f32 = 1.0; // fuel/s at full throttle (per tank)
const ROT_FORCE: f32 = 15.0; // rad/s

const MOON_SURFACE_GRAVITY: f32 = PLANET_SURFACE_GRAVITY / 6.0;
const MOON_MASS: f32 = MOON_SURFACE_GRAVITY * (MOON_RADIUS * MOON_RADIUS) / G; // G·M_moon = 1·10⁶
const MOON_RADIUS: f32 = 200.0;
const MOON_ORBIT: f32 = 10000.0; // distance from planet center

const LANDING_MAX_SPEED: f32 = 400.0; // max speed (or relative speed) for a safe landing
const STAGE_SEP_VEL: f32 = 10.0;

// ── Game constants ────────────────────────────────────────────────────────────────
const DEFAULT_SCALE: f32 = 0.5;
const MAP_VIEW_SCALE: f32 = 25.0;

const FUEL_TANK_SIZE: Vec2 = Vec2::new(0.0, 16.0);
const ENGINE_SIZE: Vec2 = Vec2::new(0.0, 6.0);
const POD_BOTTOM_Y: f32 = -6.0;
const STAGE_GAP: f32 = 0.0;
const DEFAULT_FUEL_PER_TANK: f32 = 40.0;
const DEFAULT_THRUST: f32 = PLANET_SURFACE_GRAVITY * 1.3;

// ── Mass constants (arbitrary units, just need consistent ratios) ─────────────
const POD_MASS: f32 = 5.0;
const ENGINE_MASS: f32 = 3.0;
const FUEL_TANK_DRY_MASS: f32 = 1.0; // empty tank shell
const FUEL_DENSITY: f32 = 0.5; // mass per unit of fuel

// ── Animation constants ────────────────────────────────────────────────────────────────
const SPRITE_POD: usize = 0;
const SPRITE_FUEL: usize = 16;
const SPRITE_ENGINE: usize = 32;
const SPRITE_EXHAUST_START: usize = 48;
const SPRITE_EXHUAST_END: usize = 50;

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component, Default)]
struct Rocket {
    throttle: f32, // 0 or 1
    torque: f32,   // -1 or 0 or 1 rotation
    crashed: bool,
    landed: bool,
    landed_body: Option<Entity>, // which body we're on (None when flying)
    body_offset: Vec2,           // surface-normal offset from that body's center
    active_stage: Option<Entity>, // currently burning stage
    stage_queue: Vec<Entity>,    // remaining stages, front = next to activate
    total_mass: f32,
}

/// Build a Quat representing a rocket pointing along `direction` (a unit Vec2).
/// The sprite sheet draws the rocket pointing right (+X), so we rotate it to face
/// the desired direction by computing the CCW angle from +X.
fn quat_from_dir(direction: Vec2) -> Quat {
    Quat::from_rotation_z(direction.to_angle() - FRAC_PI_2)
}

#[derive(Component)]
struct PlayerRocket;

#[derive(Component)]
struct CelestialBody {
    mass: f32,
    radius: f32,
    velocity: Vec2,
    fixed: bool, // if true, unaffected by gravity (e.g. the central planet)
}

impl CelestialBody {
    pub fn gravity_at(&self, dist: Vec2) -> f32 {
        let mut dist_sq = dist.length_squared();
        if dist_sq < 1.0 {
            dist_sq = 1.0;
        }
        // let dist = dist_sq.sqrt();
        G * self.mass / dist_sq
    }
}

#[derive(Component)]
struct CommandPod;

// ── Stage components ───────────────────────────────────────────────────────────

#[derive(Component)]
struct RocketStage;

#[derive(Component)]
struct FuelTank {
    fuel: f32,
    capacity: f32,
}

#[derive(Component)]
struct Engine {
    thrust: f32, // units/s²
}

// HUD label — one enum covers all telemetry rows; add a variant to add a new row
#[derive(Component, PartialEq, Eq)]
enum HudLabel {
    Alt,
    Vel,
    Fuel,
    Status,
}
#[derive(Component)]
struct HudFps;

// ── Editor components ─────────────────────────────────────────────────────────

#[derive(Component)]
struct EditorRoot;

#[derive(Component)]
struct StageButton(Entity);

#[derive(Component)]
struct AddStageButton;

// ── Resources ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct MapView(bool);

#[derive(Resource)]
struct RocketAssets {
    command_pod_sprite: Sprite,
    tank_sprite: Sprite,
    engine_sprite: Sprite,
    exhaust: Sprite,
}

// ── App ───────────────────────────────────────────────────────────────────────

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
struct PlanetMaterial {
    #[uniform(0)]
    params: PlanetMaterialUniform,
}

impl Material2d for PlanetMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/planet.wgsl".into()
    }
}

fn animate_planet_time(time: Res<Time>, mut planet_materials: ResMut<Assets<PlanetMaterial>>) {
    for (_, mat) in planet_materials.iter_mut() {
        mat.params.time += time.delta_secs();
    }
}

fn main() {
    App::new()
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(
            PhysicsPlugins::default(), // Enables debug rendering
        )
        // .add_plugins(PhysicsDebugPlugin::default())
        .insert_resource(Gravity(Vec2::ZERO)) // we apply custom N-body gravity manually
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Space IO – 2D KSP Prototype".into(),
                        resolution: (1280., 720.).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(Material2dPlugin::<PlanetMaterial>::default())
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.08)))
        .insert_resource(MapView::default())
        .add_systems(Startup, (setup, apply_deferred, relayout_rocket).chain())
        .add_systems(Update, animate_planet_time)
        .add_systems(
            Update,
            (
                handle_input,
                handle_editor_input,
                update_bodies,
                // physics_step,
                // handle_planet_collision,
                // check_surface_contact,
                // relayout_rocket,
                collision_handler,
                rebuild_editor_ui,
                update_trajectory,
                update_exhaust,
                animate_sprite,
                // follow_camera,
                update_hud,
                update_fps,
            )
                .chain(),
        )
        .add_systems(FixedUpdate, (physics_step, follow_camera))
        // .add_systems(PostUpdate, follow_camera.before(TransformSystem::TransformPropagate))
        .run();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a single rocket stage with `fuel_count` tanks, an engine, optional
/// decoupler, and an exhaust flame. Transforms start at default — call
/// `relayout_rocket` to position everything correctly.
fn build_stage(
    commands: &mut Commands,
    rocket_assets: &RocketAssets,
    fuel_count: u32,
    fuel_per_tank: f32,
    thrust: f32,
) -> Entity {
    commands
        .spawn((RocketStage, Transform::default(), Visibility::default()))
        .with_children(|p| {
            for _ in 0..fuel_count {
                p.spawn((
                    rocket_assets.tank_sprite.clone(),
                    Transform::default(),
                    FuelTank {
                        fuel: fuel_per_tank,
                        capacity: fuel_per_tank,
                    },
                    Collider::rectangle(12., 5.0),
                    Mass(10.0),
                ));
            }
            p.spawn((
                rocket_assets.engine_sprite.clone(),
                Transform::default(),
                Engine { thrust },
                Collider::rectangle(12., 2.0),
                Mass(10.0),
            ))
            .with_child((
                rocket_assets.exhaust.clone(),
                Transform::from_xyz(0.0, -3.0 - 8.0, 0.0),
                AnimationIndices {
                    first: SPRITE_EXHAUST_START,
                    last: SPRITE_EXHUAST_END,
                },
                AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
            ));
        })
        .id()
}

/// Spawn the default two-stage rocket.
fn spawn_default_rocket(commands: &mut Commands, assets: &RocketAssets) {
    let stage1 = build_stage(commands, assets, 1, 80.0, PLANET_SURFACE_GRAVITY * 1.5);
    let stage2 = build_stage(commands, assets, 1, 40.0, PLANET_SURFACE_GRAVITY * 1.3);

    let pod = commands
        .spawn((
            assets.command_pod_sprite.clone(),
            Transform::default(),
            Collider::rectangle(12., 2.0),
            Mass(10.0),
            CommandPod,
        ))
        .id();

    commands
        .spawn((
            Transform::from_xyz(0., PLANET_RADIUS * 1.05, 1.0),
            Visibility::default(),
            Rocket {
                active_stage: Some(stage1),
                stage_queue: vec![stage2],
                ..Default::default()
            },
            PlayerRocket,
            // avian2d physics
            RigidBody::Dynamic,
            // Collider::rectangle(12., 32.0),
            // Collider::circle(40.0),
            ExternalForce::new(Vec2::ZERO).with_persistence(false),
            ExternalTorque::new(0.0).with_persistence(false),
            Restitution::new(0.4),
            Friction::new(0.9),
            // TransformInterpolation,
            // Sensor means we detect collisions but avian doesn't apply forces;
            // our game logic (handle_planet_collision) decides land vs crash.
            // Sensor,
            // Rotation is controlled by player input; prevent avian from touching it.
            // LockedAxes::ROTATION_LOCKED,
        ))
        .add_child(pod)
        .add_child(stage1)
        .add_child(stage2);
}

// ── Animation ─────────────────────────────────────────────────────────────────────
#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = if atlas.index >= indices.last {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let r0 = PLANET_RADIUS; // + START_HEIGHT; // 280 – initial orbit radius
                            // let orbital_v = 0.; //(G * PLANET_MASS / r0).sqrt(); // ≈ 267 units/s

    // assert!(MOON_SURFACE_GRAVITY < PLANET_SURFACE_GRAVITY);

    // Camera – start centered on the rocket
    commands.spawn((Camera2d, Transform::from_xyz(0., r0, 0.)));

    // ── Stars ──────────────────────────────────────────────────────────────
    let mut rng = rand::thread_rng();
    for _ in 0..300 {
        let x: f32 = rng.gen_range(-3000.0..3000.0);
        let y: f32 = rng.gen_range(-3000.0..3000.0);
        let r: f32 = rng.gen_range(0.5..2.0);
        let b: f32 = rng.gen_range(0.5..1.0);
        commands.spawn((
            Mesh2d(meshes.add(Circle::new(r))),
            MeshMaterial2d(materials.add(Color::srgba(b, b, b, 0.85))),
            Transform::from_xyz(x, y, -1.0),
        ));
    }

    // ── Planet ─────────────────────────────────────────────────────────────
    // Colors from Rivers.tscn default scheme:
    //   [0] bright green, [1] mid green, [2] dark teal, [3] shadow blue-grey
    //   [4] shallow river blue, [5] deep river blue
    #[rustfmt::skip]
    let planet_mat = planet_materials.add(PlanetMaterial {
        params: PlanetMaterialUniform {
            colors: [
                Vec4::new(0.388, 0.671, 0.247, 1.0),
                Vec4::new(0.231, 0.490, 0.310, 1.0),
                Vec4::new(0.184, 0.341, 0.325, 1.0),
                Vec4::new(0.157, 0.208, 0.251, 1.0),
                Vec4::new(0.310, 0.643, 0.722, 1.0),
                Vec4::new(0.251, 0.286, 0.451, 1.0),
            ],
            light_origin:   Vec2::new(0.39, 0.39),
            pixels:         100.0,
            rotation:       0.2,
            time_speed:     0.1,
            dither_size:    3.951,
            light_border_1: 0.287,
            light_border_2: 0.476,
            river_cutoff:   0.368,
            size:           4.6,
            seed:           rand::thread_rng().gen_range(1.0f32..10.0f32),
            octaves:        6,
            time:           0.0,
            should_dither:  1,
            _pad0:          0,
            _pad1:          0,
        },
    });
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(PLANET_RADIUS))),
        MeshMaterial2d(planet_mat),
        Transform::default(),
        CelestialBody {
            mass: PLANET_MASS,
            radius: PLANET_RADIUS,
            velocity: Vec2::ZERO,
            fixed: true,
        },
        // avian2d: static body with a circular collider for rocket landing detection
        RigidBody::Static,
        Collider::circle(PLANET_RADIUS),
    ));

    // Atmosphere glow ring (visual only, no physics)
    commands.spawn((
        Mesh2d(meshes.add(Annulus::new(PLANET_RADIUS + 1.0, PLANET_RADIUS * 1.1))),
        MeshMaterial2d(materials.add(Color::srgba(0.4, 0.7, 1.0, 0.10))),
        Transform::from_xyz(0., 0., 0.05),
    ));

    // ── Moon ───────────────────────────────────────────────────────────────
    let moon_orbital_v = (G * PLANET_MASS / MOON_ORBIT).sqrt();
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(MOON_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgb(0.62, 0.62, 0.68))),
        Transform::from_xyz(MOON_ORBIT, 0., 0.2),
        CelestialBody {
            mass: MOON_MASS,
            radius: MOON_RADIUS,
            velocity: Vec2::new(0., -moon_orbital_v),
            fixed: false,
        },
    ));

    // ── Rocket ─────────────────────────────────────────────────────────────
    let texture = asset_server.load("space-io.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(16), 16, 16, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    let rocket_assets = RocketAssets {
        command_pod_sprite: Sprite::from_atlas_image(
            texture.clone(),
            TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: SPRITE_POD,
            },
        ),
        tank_sprite: Sprite {
            image: texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: SPRITE_FUEL,
            }),
            ..Default::default()
        },
        engine_sprite: Sprite {
            image: texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: SPRITE_ENGINE,
            }),
            rect: Some(Rect::new(0.0, 0.0, 16.0, 6.0)),
            ..Default::default()
        },
        exhaust: Sprite {
            image: texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: SPRITE_EXHAUST_START,
            }),
            color: Color::NONE,
            ..Default::default()
        },
    };
    spawn_default_rocket(&mut commands, &rocket_assets);
    commands.insert_resource(rocket_assets);

    // ── HUD ────────────────────────────────────────────────────────────────
    let mono = TextFont {
        font_size: 18.0,
        ..default()
    };

    commands.spawn((
        Text::new("ALT  --------"),
        mono.clone(),
        TextColor(Color::srgb(0.55, 1.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
        HudLabel::Alt,
    ));
    commands.spawn((
        Text::new("VEL  --------"),
        mono.clone(),
        TextColor(Color::srgb(0.55, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(34.),
            left: Val::Px(12.),
            ..default()
        },
        HudLabel::Vel,
    ));
    commands.spawn((
        Text::new("FUEL --------"),
        mono.clone(),
        TextColor(Color::srgb(1.0, 0.82, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(56.),
            left: Val::Px(12.),
            ..default()
        },
        HudLabel::Fuel,
        Interaction::default(),
    ));
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.35, 0.35)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(82.),
            left: Val::Px(12.),
            ..default()
        },
        HudLabel::Status,
    ));
    commands.spawn((
        Text::new("FPS --"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1., 1., 1., 0.4)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            right: Val::Px(12.),
            ..default()
        },
        HudFps,
    ));
    // Controls hint (bottom-left)
    commands.spawn((
        Text::new("A/D: rotate     W/↑: thrust     SPACE: stage     M: map     R: reset"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgba(1., 1., 1., 0.35)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
    ));
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    assets: Res<RocketAssets>,
    mut commands: Commands,
    mut q: Query<
        (
            Entity,
            &mut Transform,
            &mut Rocket,
            // &mut Position,
            &mut LinearVelocity,
        ),
        With<PlayerRocket>,
    >,
    mut map_view: ResMut<MapView>,
    bodies: Query<(&Transform, &CelestialBody), Without<Rocket>>,
) {
    let Ok((rocket_entity, mut tf, mut rocket, mut lin_vel)) = q.get_single_mut()
    else {
        return;
    };

    // Map view toggle
    if keys.just_pressed(KeyCode::KeyM) {
        map_view.0 = !map_view.0;
    }

    // Reset — despawn current Rocket and spawn new one
    if keys.just_pressed(KeyCode::KeyR) {
        commands.entity(rocket_entity).despawn_recursive();

        spawn_default_rocket(&mut commands, &assets);
        return;
    }

    if rocket.crashed {
        return;
    }

    // Stage separation — Create new rocket with this stage
    if keys.just_pressed(KeyCode::Space) {
        // Note: This does not change the transform, not sure if that is an issue or not, now the stage's origin may be in a weird place
        if let Some(current) = rocket.active_stage.take() {
            let nose = tf.local_y().truncate();
            let sep_vel = **lin_vel - STAGE_SEP_VEL * nose;
            commands
                .spawn((
                    Visibility::default(),
                    tf.clone(),
                    Rocket {
                        active_stage: Some(current),
                        ..Default::default()
                    },
                    // give the separated stage its own physics body
                    RigidBody::Dynamic,
                    LinearVelocity(sep_vel),
                    // Avian's update_collider_parents only processes entities that have
                    // both RigidBody and AncestorMarker<ColliderMarker>. When we reparent
                    // the stage subtree here, the OnAdd<ColliderMarker> observer that
                    // normally adds this marker walks UP the chain but stops at the stage
                    // entity (which already has the marker), so it never reaches this new
                    // root. We insert it manually so the next FixedPostUpdate correctly
                    // re-registers all descendant colliders to this rigid body.
                    AncestorMarker::<ColliderMarker>::default(),
                ))
                .add_child(current);
        }
        rocket.active_stage = if !rocket.stage_queue.is_empty() {
            Some(rocket.stage_queue.remove(0))
        } else {
            None
        };
        return;
    }

    // Rotation is always allowed — player aims before launching
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        rocket.torque = 1.0;
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        rocket.torque = -1.0;
    } else {
        rocket.torque = 0.0;
    }

    let thrusting = keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW);

    if rocket.landed {
        if thrusting {
            rocket.landed = false;
            rocket.throttle = 1.0;
        }
    } else {
        rocket.throttle = if thrusting { 1.0 } else { 0.0 };
    }

    // debugging tools
    if keys.just_pressed(KeyCode::Digit1) {
        // go back to planet
        let new_pos = Vec2::new(0., PLANET_RADIUS);
        tf.translation = Vec3::new(new_pos.x, new_pos.y, 1.0);
        // avian_pos.0 = new_pos;
        *lin_vel = LinearVelocity(Vec2::ZERO);
        rocket.landed = false;
        rocket.crashed = false;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        // go to orbit
        let r0 = PLANET_RADIUS * 1.1;
        let orbital_v = (G * PLANET_MASS / r0).sqrt();
        let new_pos = Vec2::new(0., r0);
        tf.translation = Vec3::new(new_pos.x, new_pos.y, 1.0);
        // avian_pos.0 = new_pos;
        lin_vel.0 = Vec2::new(orbital_v, 0.0);
        rocket.landed = false;
        rocket.crashed = false;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        // go to moon orbit
        let (moon_tf, moon) = bodies.iter().find(|body| body.1.fixed == false).unwrap();
        let r0 = MOON_RADIUS * 1.1;
        let orbital_v = (G * MOON_MASS / r0).sqrt();
        let new_pos = Vec2::new(moon_tf.translation.x, moon_tf.translation.y + r0);
        tf.translation = Vec3::new(new_pos.x, new_pos.y, 1.0);
        // avian_pos.0 = new_pos;
        lin_vel.0 = moon.velocity + Vec2::new(orbital_v, 0.0);
        rocket.landed = false;
        rocket.crashed = false;
    }
}

// TODO: N-body is overkill here, let's just do the parent LOL
fn update_bodies(time: Res<Time>, mut bodies: Query<(Entity, &mut Transform, &mut CelestialBody)>) {
    let dt = time.delta_secs();

    // Snapshot positions/masses so we can borrow mutably below
    let states: Vec<(Entity, Vec2, f32)> = bodies
        .iter()
        .map(|(e, tf, b)| (e, tf.translation.truncate(), b.mass))
        .collect();

    for (entity, mut tf, mut body) in bodies.iter_mut() {
        if body.fixed {
            continue;
        }
        let pos = tf.translation.truncate();
        let mut accel = Vec2::ZERO;
        for &(other_entity, other_pos, other_mass) in &states {
            if other_entity == entity {
                continue;
            }
            let to_other = other_pos - pos;
            let dist_sq = to_other.length_squared();
            if dist_sq < 1.0 {
                continue;
            }
            let dist = dist_sq.sqrt();
            accel += (to_other / dist) * (G * other_mass / dist_sq);
            // accel +=
        }
        body.velocity += accel * dt;
        let v = body.velocity;
        tf.translation.x += v.x * dt;
        tf.translation.y += v.y * dt;
    }
}

fn physics_step(
    time: Res<Time>,
    bodies: Query<(Entity, &Transform, &CelestialBody)>,
    mut rocket_q: Query<
        (
            &Transform,
            &ComputedMass,
            &mut Rocket,
            &mut ExternalForce,
            &mut ExternalTorque,
        ),
        Without<CelestialBody>,
    >,
    stage_q: Query<&Children, With<RocketStage>>,
    engine_q: Query<&Engine>,
    mut tank_q: Query<&mut FuelTank>,
) {
    for (tf, mass, rocket, mut ext_force, mut ext_torque) in rocket_q.iter_mut() {
        if rocket.crashed {
            continue;
        }

        // While landed, stay stationary (or match body velocity for moving bodies)
        // if rocket.landed {
        //     if let Some(e) = rocket.landed_body {
        //         if let Ok((_, _body_tf, body)) = bodies.get(e) {
        //             rocket.velocity = body.velocity;
        //             lin_vel.0 = body.velocity;
        //         }
        //     }
        //     continue;
        // }

        let dt = time.delta_secs();
        let pos = tf.translation.truncate();

        let mut force: Vec2 = Vec2::ZERO;

        // Gravity from every celestial body
        for (_, body_tf, body) in bodies.iter() {
            let to_body = body_tf.translation.truncate() - pos;
            let dist_sq = to_body.length_squared();
            if dist_sq < 1.0 {
                continue;
            }
            let dist = dist_sq.sqrt();
            force += (to_body / dist) * (G * body.mass / dist_sq);
        }

        // Thrust — read engine thrust and consume fuel from the active stage's children
        if rocket.throttle > 0.0 {
            if let Some(stage_entity) = rocket.active_stage {
                let children: Vec<Entity> = stage_q
                    .get(stage_entity)
                    .map(|c| c.iter().copied().collect())
                    .unwrap_or_default();

                let total_thrust: f32 = children
                    .iter()
                    .filter_map(|&c| engine_q.get(c).ok())
                    .map(|e| e.thrust)
                    .sum();

                // Collect total fuel (immutable borrows released after sum())
                let total_fuel: f32 = children
                    .iter()
                    .filter_map(|&c| tank_q.get(c).ok())
                    .map(|t| t.fuel)
                    .sum();

                if total_thrust > 0.0 && total_fuel > 0.0 {
                    let nose = tf.local_y().truncate();
                    force += nose * total_thrust;

                    // Drain each tank equally
                    for &child in &children {
                        if let Ok(mut tank) = tank_q.get_mut(child) {
                            tank.fuel = (tank.fuel - FUEL_RATE * dt).max(0.0);
                        }
                    }
                }
            }
        }

        ext_force.set_force(force * mass.value());

        let torque: f32 = rocket.torque * 2000.0;

        ext_torque.set_torque(torque);
        // log::dbg
    }
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut q: Query<&mut Text, With<HudFps>>) {
    let Ok(mut text) = q.get_single_mut() else {
        return;
    };
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        *text = Text::new(format!("{fps:.0} FPS"));
    }
}

struct OrbitalParameters {
    a: f32,       // semi-major axis
    b: f32,       // semi-minor axis
    ecc: f32,     // eccentricity
    center: Vec2, // center of orbit
    rot: Vec2,    // rotation aligning the loacl +X axis with the periapsis direction
}

impl OrbitalParameters {
    pub fn calc(mass: f32, pos: Vec2, vel: Vec2) -> Option<Self> {
        let mu = G * mass;

        let r = pos.length().max(1.0);

        // Specific orbital energy: ε = v²/2 − μ/r
        let energy = 0.5 * vel.length_squared() - mu / r;

        // Specific angular momentum (scalar Z component of r × v)
        let h = pos.x * vel.y - pos.y * vel.x;
        if h.abs() < 0.1 {
            return None; // near-radial free-fall, skip
        }

        if energy >= 0.0 {
            // Escape / hyperbolic trajectory — no closed ellipse to draw
            return None;
        }

        // Eccentricity vector: points from focus toward periapsis, magnitude = eccentricity
        // In 2D: (v × h) / μ − r̂,  where v × h = Vec2(vy·h, −vx·h)
        let e_vec = Vec2::new(vel.y * h, -vel.x * h) / mu - pos / r;
        let ecc = e_vec.length().clamp(0.0, 0.9999);

        // Orbital elements
        let a = -mu / (2.0 * energy); // semi-major axis
        let b = a * (1.0 - ecc * ecc).sqrt(); // semi-minor axis

        // Ellipse center: displaced from the focus (planet) opposite the eccentricity direction
        let e_hat = if ecc > 1e-6 { e_vec / ecc } else { Vec2::X };
        let center = -e_hat * (a * ecc);

        // Rotation aligning the local +X axis with the periapsis direction
        let rot = Vec2::from_angle(e_hat.y.atan2(e_hat.x));
        Some(OrbitalParameters {
            a,
            b,
            ecc,
            center,
            rot,
        })
    }

    pub fn periapsis(&self) -> f32 {
        self.a * (1.0 - self.ecc)
    }
}

fn draw_orbit(gizmos: &mut Gizmos, focus: Vec2, orbit: OrbitalParameters) {
    let periapsis = orbit.periapsis();
    let color = if periapsis <= PLANET_RADIUS {
        Color::srgba(1.0, 0.4, 0.2, 0.6) // impact orbit
    } else {
        Color::srgba(0.4, 0.9, 1.0, 0.55) // safe orbit
    };

    // Sample the ellipse and draw as a closed line strip
    const SEGMENTS: usize = 128;
    let points: Vec<Vec2> = (0..=SEGMENTS)
        .map(|i| {
            let theta = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            focus
                + orbit.center
                + orbit
                    .rot
                    .rotate(Vec2::new(orbit.a * theta.cos(), orbit.b * theta.sin()))
        })
        .collect();

    gizmos.linestrip_2d(points, color);
}

fn update_trajectory(
    rocket_q: Query<(&Transform, &LinearVelocity, &Rocket), With<PlayerRocket>>,
    bodies: Query<(&Transform, &CelestialBody)>,
    mut gizmos: Gizmos,
) {
    // draw moon trajectory
    for (bt, body) in bodies.iter() {
        if body.fixed {
            continue;
        }
        for (bt2, body2) in bodies.iter() {
            if !body2.fixed {
                continue;
            }

            let pos = bt.translation.truncate() - bt2.translation.truncate(); // position relative to orbital focus
            let vel = body.velocity;

            if let Some(orbit) = OrbitalParameters::calc(body2.mass, pos, vel) {
                draw_orbit(&mut gizmos, bt2.translation.truncate(), orbit);
            }
        }
    }

    let Ok((rtf, velocity, rocket)) = rocket_q.get_single() else {
        return;
    };
    if rocket.crashed || rocket.landed {
        return;
    }

    // Use the body with the highest gravitational force
    // TODO: THIS DOESN"T WORK
    let Some((focus, mass)) = bodies
        .iter()
        // .filter(|(_, b)| b.fixed)
        .max_by(|(at, a), (bt, b)| {
            let a_grav = a.gravity_at(at.translation.truncate() - rtf.translation.truncate());
            let b_grav = b.gravity_at(bt.translation.truncate() - rtf.translation.truncate());
            // bt.translation.truncate() - at.translation.truncate()
            a_grav.partial_cmp(&b_grav).unwrap()
        })
        .map(|(tf, b)| (tf.translation.truncate(), b.mass))
    else {
        return;
    };

    let pos = rtf.translation.truncate() - focus; // position relative to orbital focus
    let vel = velocity;

    if let Some(orbit) = OrbitalParameters::calc(mass, pos, **vel) {
        draw_orbit(&mut gizmos, focus, orbit);
    }
}

/// Handles rocket collisions with the planet (detected by avian2d).
/// The planet has a Collider::circle so avian fires CollisionStarted events when
/// a rocket's circle collider overlaps it. We decide land vs crash here.
fn collision_handler(
    mut commands: Commands,
    mut collision_events: EventReader<Collision>,
    planet_q: Query<(Entity, &CelestialBody), With<RigidBody>>,
) {
    for Collision(contacts) in collision_events.read() {

        // TODO: If normal_impusle and tangent_impulse are less than something, switch to landed
        // landed state: Should be marked not active to the physics system, but if an active ship gets close enough, will need to be re-activated

        // FOR NOW: Global break threshold
        const BREAK_THRESHOLD: f32 = 200.0;
        if contacts.total_normal_impulse < BREAK_THRESHOLD && contacts.total_tangent_impulse < BREAK_THRESHOLD {
            continue;
        }


        // TODO: if this is a command pod, that's game over
        // only if not planet
        if planet_q.get(contacts.entity1).is_err() {
            commands.entity(contacts.entity1).despawn_recursive();
        }
        // only if not planet
        if planet_q.get(contacts.entity2).is_err() {
            commands.entity(contacts.entity2).despawn_recursive();
        }
    }
}

/// Manual collision check for non-physics bodies (the moon has no avian Collider).
/// Also acts as a fallback for any rocket that avian hasn't detected yet.
// fn check_surface_contact(
//     mut rocket_q: Query<(&mut Transform, &mut Rocket, &mut LinearVelocity), Without<CelestialBody>>,
//     bodies: Query<(Entity, &Transform, &CelestialBody)>,
// ) {
//     for (mut rtf, mut rocket, mut lin_vel) in rocket_q.iter_mut() {
//         if rocket.crashed || rocket.landed {
//             continue;
//         }

//         let pos = rtf.translation.truncate();

//         for (entity, body_tf, body) in bodies.iter() {
//             // Skip fixed bodies — they're handled by handle_planet_collision via avian events
//             if body.fixed {
//                 continue;
//             }

//             let body_pos = body_tf.translation.truncate();
//             let from_body = pos - body_pos;
//             if from_body.length() > body.radius + 2.0 {
//                 continue;
//             }

//             // Ignore contact if the rocket is already moving away from the surface
//             let rel_vel = rocket.velocity - body.velocity;
//             if rel_vel.dot(from_body.normalize()) > 0.0 {
//                 continue;
//             }

//             let rel_speed = rel_vel.length();
//             if rel_speed <= LANDING_MAX_SPEED {
//                 let normal = from_body.normalize();
//                 rocket.velocity = body.velocity;
//                 lin_vel.0 = body.velocity;
//                 rocket.throttle = 0.0;
//                 rocket.landed = true;
//                 rocket.landed_body = Some(entity);
//                 rocket.body_offset = normal * body.radius;
//                 rtf.translation = (body_pos + rocket.body_offset).extend(1.0);
//                 rtf.rotation = quat_from_dir(normal);
//             } else {
//                 rocket.crashed = true;
//                 rocket.velocity = Vec2::ZERO;
//                 lin_vel.0 = Vec2::ZERO;
//                 rocket.throttle = 0.0;
//             }
//             break; // only one contact at a time
//         }
//     }
// }

fn update_exhaust(
    rocket_q: Query<&Rocket>,
    stage_q: Query<&Children, With<RocketStage>>,
    stage_parent_q: Query<&Parent, With<RocketStage>>,
    tank_q: Query<&FuelTank>,
    mut sprite_q: Query<&mut Sprite, Without<Engine>>,
    mut engine_q: Query<(&Parent, &Children), With<Engine>>,
) {
    for (parent, children) in engine_q.iter_mut() {
        let stage_entity = parent.get();
        let Ok(rocket) = stage_parent_q
            .get(stage_entity)
            .and_then(|p| rocket_q.get(p.get()))
        else {
            continue;
        };

        let is_active = rocket.active_stage == Some(stage_entity);
        let has_fuel = stage_q
            .get(stage_entity)
            .map(|children| {
                children
                    .iter()
                    .any(|&c| tank_q.get(c).map(|t| t.fuel > 0.0).unwrap_or(false))
            })
            .unwrap_or(false);

        // TODO: Move this calculation to physics step!
        let active = rocket.throttle > 0.0 && is_active && has_fuel;

        let mut sprite = sprite_q.get_mut(children[0]).unwrap();
        if active {
            sprite.color = Color::default();
        } else {
            sprite.color = Color::NONE;
        }
    }
}

fn follow_camera(
    rocket_q: Query<&Transform, (With<Rocket>, With<PlayerRocket>)>,
    mut cam_q: Query<&mut Transform, (With<Camera2d>, Without<Rocket>)>,
    mut proj_q: Query<&mut OrthographicProjection, With<Camera2d>>,
    map_view: Res<MapView>,
    time: Res<Time>,
) {
    let Ok(rtf) = rocket_q.get_single() else {
        return;
    };
    let Ok(mut ctf) = cam_q.get_single_mut() else {
        return;
    };
    let Ok(mut proj) = proj_q.get_single_mut() else {
        return;
    };

    let dt = time.delta_secs();
    let target_pos = Vec3::new(rtf.translation.x, rtf.translation.y, ctf.translation.z);
    let target_scale = if map_view.0 {
        MAP_VIEW_SCALE
    } else {
        DEFAULT_SCALE
    };

    ctf.translation = ctf.translation.lerp(target_pos, 6.0 * dt);
    proj.scale += (target_scale - proj.scale) * (6.0 * dt).min(1.0);
}

fn update_hud(
    rocket_q: Query<(&Transform, &LinearVelocity, &Rocket), With<PlayerRocket>>,
    stage_q: Query<&Children, With<RocketStage>>,
    tank_q: Query<&FuelTank>,
    mut hud_q: Query<(&HudLabel, &mut Text)>,
) {
    let Ok((tf, velocity, rocket)) = rocket_q.get_single() else {
        return;
    };
    let alt = (tf.translation.truncate().length() - PLANET_RADIUS).max(0.0);
    let speed = velocity.length();
    // let
    let (stage_fuel, stage_capacity): (f32, f32) = rocket
        .active_stage
        .and_then(|se| stage_q.get(se).ok())
        .map(|children| {
            children
                .iter()
                .filter_map(|&c| tank_q.get(c).ok())
                .fold((0.0, 0.0), |(f, cap), t| (f + t.fuel, cap + t.capacity))
        })
        .unwrap_or((0.0, 1.0));

    for (label, mut text) in &mut hud_q {
        *text = Text::new(match label {
            HudLabel::Alt => format!("ALT  {:>8.0} m", alt),
            HudLabel::Vel => format!("VEL  {:>8.1} m/s", speed),
            HudLabel::Fuel => {
                const W: usize = 10;
                let filled = ((stage_fuel / stage_capacity) * W as f32).round() as usize;
                let filled = filled.min(W);
                format!("FUEL [{}{}]", "=".repeat(filled), " ".repeat(W - filled))
            }
            HudLabel::Status => {
                if rocket.crashed {
                    "CRASHED  –  press R to reset".to_string()
                } else if rocket.landed {
                    "LANDED  –  W/↑ to launch\nfoobar".to_string()
                } else if rocket.active_stage.is_some() && stage_fuel <= 0.0 {
                    "OUT OF FUEL  –  SPACE to stage".to_string()
                } else {
                    String::new()
                }
            }
        });
    }
}

// ── Rocket layout ─────────────────────────────────────────────────────────────

/// Reposition all stages and their children so they stack neatly under the pod,
/// then shift everything so the entity origin sits at the center of mass.
fn relayout_rocket(
    mut rocket_q: Query<&mut Rocket, With<PlayerRocket>>,
    stage_q: Query<&Children, With<RocketStage>>,
    tank_q: Query<&FuelTank>,
    engine_check: Query<(), With<Engine>>,
    pod_q: Query<Entity, With<CommandPod>>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(mut rocket) = rocket_q.get_single_mut() else {
        return;
    };

    // Walk stages top-to-bottom (stage_queue reversed, then active_stage).
    let all_stages: Vec<Entity> = rocket
        .stage_queue
        .iter()
        .copied()
        .rev()
        .chain(rocket.active_stage)
        .collect();

    // ── Pass 1: layout positions relative to the pod centre (y=0) ─────────

    // Pod centre sits at local y=0 initially; the triangle spans -9..+10
    let pod_y: f32 = 0.0;

    // Accumulate (world_y, mass) pairs for CoM calculation
    let mut mass_items: Vec<(f32, f32)> = vec![(pod_y, POD_MASS)];

    let mut cursor_y = POD_BOTTOM_Y;

    // For each stage, record the absolute y of each part (relative to rocket entity)
    struct StageLayout {
        stage_entity: Entity,
        stage_y: f32,
        tank_ys: Vec<(Entity, f32, f32)>, // (entity, local_y_in_stage, mass)
        engine_y: f32,
    }

    let mut layouts: Vec<StageLayout> = Vec::new();

    for stage_entity in &all_stages {
        let Ok(children) = stage_q.get(*stage_entity) else {
            continue;
        };

        let mut tanks: Vec<(Entity, f32)> = Vec::new();
        let mut engines: Vec<Entity> = Vec::new();

        for &child in children.iter() {
            if tank_q.get(child).is_ok() {
                tanks.push((child, 0.0));
            } else if engine_check.get(child).is_ok() {
                engines.push(child);
            }
        }
        let tank_count = tanks.len();

        cursor_y -= STAGE_GAP;
        let stage_y = cursor_y;

        // Position tanks within the stage (local coords)
        for (i, (entity, ref mut local_y)) in tanks.iter_mut().enumerate() {
            *local_y = -(FUEL_TANK_SIZE.y / 2.0) - i as f32 * FUEL_TANK_SIZE.y;

            // Absolute y for CoM: stage_y + local_y
            let abs_y = stage_y + *local_y;
            let tank = tank_q.get(*entity).unwrap();
            let tank_mass = FUEL_TANK_DRY_MASS + tank.fuel * FUEL_DENSITY;
            mass_items.push((abs_y, tank_mass));
        }

        let exhaust_y = -(tank_count as f32 * FUEL_TANK_SIZE.y) - ENGINE_SIZE.y / 2.0;

        // Engine mass — positioned roughly at exhaust location
        for &_e in &engines {
            mass_items.push((stage_y + exhaust_y, ENGINE_MASS));
        }

        layouts.push(StageLayout {
            stage_entity: *stage_entity,
            stage_y,
            tank_ys: tanks
                .into_iter()
                .map(|(e, ly)| {
                    let tank = tank_q.get(e).unwrap();
                    (e, ly, FUEL_TANK_DRY_MASS + tank.fuel * FUEL_DENSITY)
                })
                .collect(),
            engine_y: exhaust_y,
        });

        cursor_y -= tank_count as f32 * FUEL_TANK_SIZE.y + ENGINE_SIZE.y;
    }

    // ── Pass 2: compute centre of mass ────────────────────────────────────

    let total_mass: f32 = mass_items.iter().map(|(_, m)| m).sum();
    rocket.total_mass = total_mass;
    let com_y = if total_mass > 0.0 {
        mass_items.iter().map(|(y, m)| y * m).sum::<f32>() / total_mass
    } else {
        0.0
    };

    // ── Pass 3: apply transforms, shifted by -com_y ───────────────────────

    // Pod
    if let Ok(pod_entity) = pod_q.get_single() {
        if let Ok(mut tf) = transforms.get_mut(pod_entity) {
            tf.translation.y = pod_y - com_y;
        }
    }

    // Stages and their children
    for layout in &layouts {
        let shifted_stage_y = layout.stage_y - com_y;

        if let Ok(mut tf) = transforms.get_mut(layout.stage_entity) {
            tf.translation.y = shifted_stage_y;
        }

        for &(tank_entity, local_y, _) in &layout.tank_ys {
            if let Ok(mut tf) = transforms.get_mut(tank_entity) {
                tf.translation.y = local_y;
            }
        }

        // engines keep their local y within the stage
        if let Ok(children) = stage_q.get(layout.stage_entity) {
            for &child in children.iter() {
                if engine_check.get(child).is_ok() {
                    if let Ok(mut tf) = transforms.get_mut(child) {
                        tf.translation.y = layout.engine_y;
                        tf.translation.z = -0.1;
                    }
                }
            }
        }
    }
}

// ── Rocket editor ─────────────────────────────────────────────────────────────

/// Spawn / despawn the editor panel. Rebuilds only when the stage layout changes.
fn rebuild_editor_ui(
    mut commands: Commands,
    rocket_q: Query<&Rocket, With<PlayerRocket>>,
    body_q: Query<&CelestialBody>,
    stage_q: Query<&Children, With<RocketStage>>,
    tank_check: Query<(), With<FuelTank>>,
    existing_editor: Query<Entity, With<EditorRoot>>,
    mut prev_state: Local<Option<(bool, Vec<(Entity, usize)>)>>,
) {
    let show_editor = rocket_q
        .get_single()
        .ok()
        .map(|rocket| {
            rocket.landed
                && rocket
                    .landed_body
                    .and_then(|e| body_q.get(e).ok())
                    .map(|b| b.fixed)
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    // Build snapshot of current stage configuration
    let current_stages: Vec<(Entity, usize)> = if show_editor {
        let rocket = rocket_q.get_single().unwrap();
        rocket
            .stage_queue
            .iter()
            .copied()
            .rev()
            .chain(rocket.active_stage)
            .map(|e| {
                let tanks = stage_q
                    .get(e)
                    .map(|ch| ch.iter().filter(|&c| tank_check.get(*c).is_ok()).count())
                    .unwrap_or(0);
                (e, tanks)
            })
            .collect()
    } else {
        vec![]
    };

    let current = (show_editor, current_stages);
    if prev_state.as_ref() == Some(&current) {
        return;
    }
    *prev_state = Some(current.clone());

    // Tear down old editor
    for e in &existing_editor {
        commands.entity(e).despawn_recursive();
    }

    if !show_editor {
        return;
    }

    let (_, ref stages) = current;

    commands
        .spawn((
            EditorRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(80.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.85)),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("ROCKET EDITOR"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // One button per stage (top-to-bottom)
            for (i, &(stage_entity, tank_count)) in stages.iter().enumerate() {
                root.spawn((
                    Text::new(format!("Stage {} [{} tanks]", i + 1, tank_count)),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.5)),
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        min_width: Val::Px(170.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                    Interaction::default(),
                    StageButton(stage_entity),
                ));
            }

            // "Add stage" button
            root.spawn((
                Text::new("+ Add Stage"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 1.0, 0.5)),
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    min_width: Val::Px(170.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.25, 0.15, 0.9)),
                Interaction::default(),
                AddStageButton,
            ));
        });
}

/// Process left-click (add tank) and right-click (remove tank / stage) on
/// editor buttons, plus the "add stage" button.
fn handle_editor_input(
    mut commands: Commands,
    rocket_assets: Res<RocketAssets>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut rocket_q: Query<(Entity, &mut Rocket), With<PlayerRocket>>,
    stage_q: Query<&Children, With<RocketStage>>,
    tank_check: Query<(), With<FuelTank>>,
    stage_buttons: Query<(&Interaction, &StageButton)>,
    add_button: Query<&Interaction, With<AddStageButton>>,
) {
    let Ok((rocket_entity, mut rocket)) = rocket_q.get_single_mut() else {
        return;
    };

    // Stage buttons — left click = add tank, right click = remove tank/stage
    for (interaction, stage_btn) in &stage_buttons {
        let stage_entity = stage_btn.0;

        // Left click: add a fuel tank
        if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
            let tank = commands
                .spawn((
                    rocket_assets.tank_sprite.clone(),
                    Transform::default(),
                    FuelTank {
                        fuel: DEFAULT_FUEL_PER_TANK,
                        capacity: DEFAULT_FUEL_PER_TANK,
                    },
                ))
                .id();
            commands.entity(stage_entity).add_child(tank);
        }

        // Right click: remove a fuel tank, or remove the whole stage if empty
        if *interaction == Interaction::Hovered && mouse.just_pressed(MouseButton::Right) {
            let Ok(children) = stage_q.get(stage_entity) else {
                continue;
            };
            let mut tank_to_remove = None;
            for &c in children.iter() {
                if tank_check.get(c).is_ok() {
                    tank_to_remove = Some(c);
                    break;
                }
            }

            if let Some(tank) = tank_to_remove {
                commands.entity(tank).despawn_recursive();
            } else {
                // No tanks left — remove the entire stage
                if rocket.active_stage == Some(stage_entity) {
                    rocket.active_stage = if !rocket.stage_queue.is_empty() {
                        Some(rocket.stage_queue.remove(0))
                    } else {
                        None
                    };
                } else {
                    rocket.stage_queue.retain(|&e| e != stage_entity);
                }
                commands.entity(stage_entity).despawn_recursive();
            }
        }
    }

    // "Add stage" button
    for interaction in &add_button {
        if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
            let new_stage = build_stage(
                &mut commands,
                &rocket_assets,
                1,
                DEFAULT_FUEL_PER_TANK,
                DEFAULT_THRUST,
            );
            commands.entity(rocket_entity).add_child(new_stage);

            // New stage goes to the bottom (fires first = active_stage)
            if let Some(old_active) = rocket.active_stage.take() {
                rocket.stage_queue.insert(0, old_active);
            }
            rocket.active_stage = Some(new_stage);
        }
    }
}
