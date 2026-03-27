//! Space IO – 2D KSP-style prototype (Bevy 0.15)
//!
//! Controls
//! --------
//!   A / ←      rotate left  (CCW)
//!   D / →      rotate right (CW)
//!   Space/↑/W  main engine  (thrust)
//!   R          reset to orbit

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, math::FloatPow, prelude::*
};
use rand::Rng;

// ── Physics constants (tuned for fun, not SI realism) ─────────────────────────
const G: f32 = 200.0;
const PLANET_MASS: f32 = 5.0e6; // G·M = 2·10⁷
const PLANET_RADIUS: f32 = 3400.0;
const PLANET_SURFACE_GRAVITY: f32 = G * PLANET_MASS / (PLANET_RADIUS * PLANET_RADIUS);
const FUEL_RATE: f32 = 15.0; // fuel/s at full throttle (per tank)
const ROT_SPEED: f32 = 2.5; // rad/s
const START_HEIGHT: f32 = 80.0; // above surface

const MOON_MASS: f32 = 10e3; // G·M_moon = 1·10⁶
const MOON_RADIUS: f32 = 200.0;
const MOON_ORBIT: f32 = 3000.0; // distance from planet center

const LANDING_MAX_SPEED: f32 = 400.0; // max speed (or relative speed) for a safe landing


// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Rocket {
    velocity: Vec2,
    angle: f32,          // radians; 0 = nose pointing +Y
    throttle: f32,       // 0 or 1
    crashed: bool,
    landed: bool,
    landed_body: Option<Entity>, // which body we're on (None when flying)
    body_offset: Vec2,           // surface-normal offset from that body's center
    active_stage: Option<Entity>, // currently burning stage
    stage_queue: Vec<Entity>,     // remaining stages, front = next to activate
}

#[derive(Component)]
struct CelestialBody {
    mass: f32,
    radius: f32,
    velocity: Vec2,
    fixed: bool, // if true, unaffected by gravity (e.g. the central planet)
}

#[derive(Component)]
struct Exhaust;

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

#[derive(Component)]
struct Decoupler; // marks a stage as separable from the one above

// HUD marker components (each unique so queries are unambiguous)
#[derive(Component)]
struct HudAlt;
#[derive(Component)]
struct HudVel;
#[derive(Component)]
struct HudFuel;
#[derive(Component)]
struct HudStatus;
#[derive(Component)]
struct HudFps;

// ── Resources ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct MapView(bool);

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Space IO – 2D KSP Prototype".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.08)))
        .insert_resource(MapView::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                update_bodies,
                physics_step,
                check_surface_contact,
                update_trajectory,
                update_exhaust,
                follow_camera,
                update_hud,
                update_fps,
            )
                .chain(),
        )
        .run();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Spawn two default stages and return (stage1_entity, stage2_entity).
/// Stage 1 (bottom): large tank + high-thrust engine + decoupler.
/// Stage 2 (top):    small tank + efficient engine.
fn spawn_default_stages(commands: &mut Commands) -> (Entity, Entity) {
    let fuel_tank_size = Vec2::new(18.0, 22.0);
    let fuel_tank_sprite = Sprite {
        color: Color::srgba(1.0, 1.0, 1.0, 1.0),
        custom_size: Some(fuel_tank_size),
        ..default()
    };

    let exhaust_sprite = Sprite {
        color: Color::srgba(1.0, 0.55, 0.05, 0.0),
        custom_size: Some(Vec2::new(7.0, 22.0)),
        ..default()
    };

    let stage1 = commands
        .spawn((RocketStage, Transform::from_xyz(0., -10., 0.), Visibility::default()))
        .with_children(|p| {
            p.spawn((fuel_tank_sprite.clone(), Transform::from_xyz(0., -44., 0.), FuelTank { fuel: 80.0, capacity: 80.0 }));
            p.spawn(Engine { thrust: PLANET_SURFACE_GRAVITY * 1.5});
            p.spawn(Decoupler);
            p.spawn((exhaust_sprite.clone(), Transform::from_xyz(0., -58., -0.1), Exhaust));
        })
        .id();

    let stage2 = commands
        .spawn((RocketStage, Transform::default(), Visibility::default()))
        .with_children(|p| {
            p.spawn((fuel_tank_sprite.clone(), Transform::from_xyz(0., -26., 0.), FuelTank { fuel: 40.0, capacity: 40.0 }));
            p.spawn(Engine { thrust: PLANET_SURFACE_GRAVITY * 1.3 });
            p.spawn((exhaust_sprite.clone(), Transform::from_xyz(0., -40., -0.1), Exhaust));
        })
        .id();

    (stage1, stage2)
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let r0 = PLANET_RADIUS; // + START_HEIGHT; // 280 – initial orbit radius
    let orbital_v = 0.; //(G * PLANET_MASS / r0).sqrt(); // ≈ 267 units/s

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
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(PLANET_RADIUS))),
        MeshMaterial2d(materials.add(Color::srgb(0.15, 0.50, 0.22))),
        Transform::default(),
        CelestialBody {
            mass: PLANET_MASS,
            radius: PLANET_RADIUS,
            velocity: Vec2::ZERO,
            fixed: true,
        },
    ));

    // Atmosphere glow ring (visual only, no physics)
    commands.spawn((
        Mesh2d(meshes.add(Annulus::new(PLANET_RADIUS + 1.0, PLANET_RADIUS *1.1))),
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

    let (stage1, stage2) = spawn_default_stages(&mut commands);

    // Rocket body (triangle; nose = top when angle == 0)
    // Stages and exhaust flame are children of this entity.
    commands.spawn((
        Mesh2d(meshes.add(Triangle2d::new(
            Vec2::new(0., 10.),   // nose
            Vec2::new(-9., -9.), // left base
            Vec2::new(9., -9.),  // right base
        ))),
        MeshMaterial2d(materials.add(Color::srgb(0.85, 0.85, 0.95))),
        Transform::from_xyz(0., r0, 1.0),
        Rocket {
            velocity: Vec2::new(orbital_v, 0.),
            angle: 0.0,
            throttle: 0.0,
            crashed: false,
            landed: false,
            landed_body: None,
            body_offset: Vec2::ZERO,
            active_stage: Some(stage1),
            stage_queue: vec![stage2],
        },
    ))
    .add_child(stage1)
    .add_child(stage2);

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
        HudAlt,
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
        HudVel,
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
        HudFuel,
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
        HudStatus,
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
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Rocket)>,
    mut map_view: ResMut<MapView>,
) {
    let Ok((rocket_entity, mut tf, mut rocket)) = q.get_single_mut() else {
        return;
    };

    // Map view toggle
    if keys.just_pressed(KeyCode::KeyM) {
        map_view.0 = !map_view.0;
    }

    // Reset — despawn current stages and respawn fresh ones
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(s) = rocket.active_stage {
            commands.entity(s).despawn_recursive();
        }
        for &s in &rocket.stage_queue {
            commands.entity(s).despawn_recursive();
        }
        let (stage1, stage2) = spawn_default_stages(&mut commands);
        commands.entity(rocket_entity).add_child(stage1).add_child(stage2);

        let r0 = PLANET_RADIUS + START_HEIGHT;
        let ov = (G * PLANET_MASS / r0).sqrt();
        *tf = Transform::from_xyz(0., r0, 1.0);
        rocket.velocity = Vec2::new(ov, 0.);
        rocket.angle = 0.0;
        rocket.throttle = 0.0;
        rocket.crashed = false;
        rocket.landed = false;
        rocket.landed_body = None;
        rocket.body_offset = Vec2::ZERO;
        rocket.active_stage = Some(stage1);
        rocket.stage_queue = vec![stage2];
        return;
    }

    if rocket.crashed {
        return;
    }

    // Stage separation — despawn the active stage and promote the next one
    if keys.just_pressed(KeyCode::Space) {
        if let Some(current) = rocket.active_stage {
            commands.entity(current).despawn_recursive();
        }
        rocket.active_stage = if !rocket.stage_queue.is_empty() {
            Some(rocket.stage_queue.remove(0))
        } else {
            None
        };
        return;
    }

    let dt = time.delta_secs();

    // Rotation is always allowed — player aims before launching
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        rocket.angle += ROT_SPEED * dt;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        rocket.angle -= ROT_SPEED * dt;
    }
    tf.rotation = Quat::from_rotation_z(rocket.angle);

    let thrusting = keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW);

    if rocket.landed {
        if thrusting {
            rocket.landed = false;
            rocket.throttle = 1.0;
        }
    } else {
        rocket.throttle = if thrusting { 1.0 } else { 0.0 };
    }
}

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
    mut rocket_q: Query<(&mut Transform, &mut Rocket), Without<CelestialBody>>,
    stage_q: Query<&Children, With<RocketStage>>,
    engine_q: Query<&Engine>,
    mut tank_q: Query<&mut FuelTank>,
) {
    let Ok((mut tf, mut rocket)) = rocket_q.get_single_mut() else {
        return;
    };
    if rocket.crashed {
        return;
    }

    // While landed, ride the body we're on (fixed bodies keep us stationary)
    if rocket.landed {
        if let Some(e) = rocket.landed_body {
            if let Ok((_, body_tf, body)) = bodies.get(e) {
                tf.translation = (body_tf.translation.truncate() + rocket.body_offset).extend(1.0);
                rocket.velocity = body.velocity;
            }
        }
        return;
    }

    let dt = time.delta_secs();
    let pos = tf.translation.truncate();

    // Gravity from every celestial body
    for (_, body_tf, body) in bodies.iter() {
        let to_body = body_tf.translation.truncate() - pos;
        let dist_sq = to_body.length_squared();
        if dist_sq < 1.0 {
            continue;
        }
        let dist = dist_sq.sqrt();
        rocket.velocity += (to_body / dist) * (G * body.mass / dist_sq) * dt;
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
                let nose = Vec2::new(-rocket.angle.sin(), rocket.angle.cos());
                rocket.velocity += nose * total_thrust * dt;

                // Drain each tank equally
                for &child in &children {
                    if let Ok(mut tank) = tank_q.get_mut(child) {
                        tank.fuel = (tank.fuel - FUEL_RATE * dt).max(0.0);
                    }
                }
            }
        }
    }

    let v = rocket.velocity;
    tf.translation.x += v.x * dt;
    tf.translation.y += v.y * dt;
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

fn update_trajectory(
    rocket_q: Query<(&Transform, &Rocket)>,
    bodies: Query<(&Transform, &CelestialBody)>,
    mut gizmos: Gizmos,
) {
    let Ok((rtf, rocket)) = rocket_q.get_single() else {
        return;
    };
    if rocket.crashed || rocket.landed {
        return;
    }

    // Use the most massive fixed body as the orbital focus
    let Some((focus, mu)) = bodies
        .iter()
        .filter(|(_, b)| b.fixed)
        .max_by(|(_, a), (_, b)| a.mass.partial_cmp(&b.mass).unwrap())
        .map(|(tf, b)| (tf.translation.truncate(), G * b.mass))
    else {
        return;
    };
    let pos = rtf.translation.truncate() - focus; // position relative to orbital focus
    let vel = rocket.velocity;
    let r = pos.length().max(1.0);

    // Specific orbital energy: ε = v²/2 − μ/r
    let energy = 0.5 * vel.length_squared() - mu / r;

    // Specific angular momentum (scalar Z component of r × v)
    let h = pos.x * vel.y - pos.y * vel.x;
    if h.abs() < 0.1 {
        return; // near-radial free-fall, skip
    }

    if energy >= 0.0 {
        // Escape / hyperbolic trajectory — no closed ellipse to draw
        return;
    }

    // Eccentricity vector: points from focus toward periapsis, magnitude = eccentricity
    // In 2D: (v × h) / μ − r̂,  where v × h = Vec2(vy·h, −vx·h)
    let e_vec = Vec2::new(vel.y * h, -vel.x * h) / mu - pos / r;
    let ecc = e_vec.length().clamp(0.0, 0.9999);

    // Orbital elements
    let a = -mu / (2.0 * energy);          // semi-major axis
    let b = a * (1.0 - ecc * ecc).sqrt();  // semi-minor axis

    // Ellipse center: displaced from the focus (planet) opposite the eccentricity direction
    let e_hat = if ecc > 1e-6 { e_vec / ecc } else { Vec2::X };
    let center = -e_hat * (a * ecc);

    // Rotation aligning the local +X axis with the periapsis direction
    let rot = Vec2::from_angle(e_hat.y.atan2(e_hat.x));

    let periapsis = a * (1.0 - ecc);
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
            focus + center + rot.rotate(Vec2::new(a * theta.cos(), b * theta.sin()))
        })
        .collect();

    gizmos.linestrip_2d(points, color);
}

fn check_surface_contact(
    mut rocket_q: Query<(&mut Transform, &mut Rocket), Without<CelestialBody>>,
    bodies: Query<(Entity, &Transform, &CelestialBody)>,
) {
    let Ok((mut rtf, mut rocket)) = rocket_q.get_single_mut() else {
        return;
    };
    if rocket.crashed || rocket.landed {
        return;
    }

    let pos = rtf.translation.truncate();

    for (entity, body_tf, body) in bodies.iter() {
        let body_pos = body_tf.translation.truncate();
        let from_body = pos - body_pos;
        if from_body.length() > body.radius + 2.0 {
            continue;
        }

        // Ignore contact if the rocket is already moving away from the surface
        let rel_vel = rocket.velocity - body.velocity;
        if rel_vel.dot(from_body.normalize()) > 0.0 {
            continue;
        }

        let rel_speed = rel_vel.length();
        if rel_speed <= LANDING_MAX_SPEED {
            let normal = from_body.normalize();
            rocket.angle = (-normal.x).atan2(normal.y);
            rocket.velocity = body.velocity;
            rocket.throttle = 0.0;
            rocket.landed = true;
            rocket.landed_body = Some(entity);
            rocket.body_offset = normal * body.radius;
            rtf.translation = (body_pos + rocket.body_offset).extend(1.0);
            rtf.rotation = Quat::from_rotation_z(rocket.angle);
        } else {
            rocket.crashed = true;
            rocket.velocity = Vec2::ZERO;
            rocket.throttle = 0.0;
        }
        break; // only one contact at a time
    }
}

fn update_exhaust(
    rocket_q: Query<&Rocket>,
    stage_q: Query<&Children, With<RocketStage>>,
    stage_parent_q: Query<&Parent, With<RocketStage>>,
    tank_q: Query<&FuelTank>,
    mut exhaust_q: Query<(&Parent, &mut Sprite), With<Exhaust>>,
) {
    for (parent, mut sprite) in exhaust_q.iter_mut() {
        let stage_entity = parent.get();
        let Ok(rocket) = stage_parent_q
            .get(stage_entity)
            .and_then(|p| rocket_q.get(p.get())) else { continue; };

        let is_active = rocket.active_stage == Some(stage_entity);
        let has_fuel = stage_q
            .get(stage_entity)
            .map(|children| {
                children
                    .iter()
                    .any(|&c| tank_q.get(c).map(|t| t.fuel > 0.0).unwrap_or(false))
            })
            .unwrap_or(false);

        let alpha = if rocket.throttle > 0.0 && is_active && has_fuel { 0.9 } else { 0.0 };
        sprite.color = Color::srgba(1.0, 0.55, 0.05, alpha);
    }
}

fn follow_camera(
    rocket_q: Query<&Transform, With<Rocket>>,
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
    let target_scale = if map_view.0 { 10.0 } else { 1.0 };

    ctf.translation = ctf.translation.lerp(target_pos, 6.0 * dt);
    proj.scale += (target_scale - proj.scale) * (6.0 * dt).min(1.0);
}

fn update_hud(
    rocket_q: Query<(&Transform, &Rocket)>,
    stage_q: Query<&Children, With<RocketStage>>,
    tank_q: Query<&FuelTank>,
    mut alt_q: Query<
        &mut Text,
        (
            With<HudAlt>,
            Without<HudVel>,
            Without<HudFuel>,
            Without<HudStatus>,
        ),
    >,
    mut vel_q: Query<
        &mut Text,
        (
            With<HudVel>,
            Without<HudAlt>,
            Without<HudFuel>,
            Without<HudStatus>,
        ),
    >,
    mut fuel_q: Query<
        &mut Text,
        (
            With<HudFuel>,
            Without<HudAlt>,
            Without<HudVel>,
            Without<HudStatus>,
        ),
    >,
    mut status_q: Query<
        &mut Text,
        (
            With<HudStatus>,
            Without<HudAlt>,
            Without<HudVel>,
            Without<HudFuel>,
        ),
    >,
) {
    let Ok((tf, rocket)) = rocket_q.get_single() else {
        return;
    };
    let alt = (tf.translation.truncate().length() - PLANET_RADIUS).max(0.0);
    let speed = rocket.velocity.length();

    let stage_fuel: f32 = rocket
        .active_stage
        .and_then(|se| stage_q.get(se).ok())
        .map(|children| {
            children
                .iter()
                .filter_map(|&c| tank_q.get(c).ok())
                .map(|t| t.fuel)
                .sum()
        })
        .unwrap_or(0.0);

    if let Ok(mut t) = alt_q.get_single_mut() {
        *t = Text::new(format!("ALT  {:>8.0} m", alt));
    }
    if let Ok(mut t) = vel_q.get_single_mut() {
        *t = Text::new(format!("VEL  {:>8.1} m/s", speed));
    }
    if let Ok(mut t) = fuel_q.get_single_mut() {
        *t = Text::new(format!("FUEL {:>8.1}", stage_fuel));
    }
    if let Ok(mut t) = status_q.get_single_mut() {
        *t = Text::new(if rocket.crashed {
            "CRASHED  –  press R to reset".to_string()
        } else if rocket.landed {
            "LANDED  –  W/↑ to launch".to_string()
        } else if rocket.active_stage.is_some() && stage_fuel <= 0.0 {
            "OUT OF FUEL  –  SPACE to stage".to_string()
        } else {
            String::new()
        });
    }
}
