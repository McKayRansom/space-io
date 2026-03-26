//! Space IO – 2D KSP-style prototype (Bevy 0.15)
//!
//! Controls
//! --------
//!   A / ←      rotate left  (CCW)
//!   D / →      rotate right (CW)
//!   Space/↑/W  main engine  (thrust)
//!   R          reset to orbit

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use rand::Rng;

// ── Physics constants (tuned for fun, not SI realism) ─────────────────────────
const G: f32 = 200.0;
const PLANET_MASS: f32 = 1.0e5; // G·M = 2·10⁷
const PLANET_RADIUS: f32 = 200.0;
const THRUST: f32 = 100.0; // units/s² at full throttle
const FUEL_RATE: f32 = 15.0; // fuel/s at full throttle
const ROT_SPEED: f32 = 2.5; // rad/s
const START_HEIGHT: f32 = 80.0; // above surface


// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
struct Rocket {
    velocity: Vec2,
    angle: f32,    // radians; 0 = nose pointing +Y
    fuel: f32,
    throttle: f32, // 0 or 1
    crashed: bool,
}

#[derive(Component)]
struct Planet;

#[derive(Component)]
struct Exhaust;

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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                physics_step,
                check_crash,
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

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let r0 = PLANET_RADIUS + START_HEIGHT; // 280 – initial orbit radius
    let orbital_v = (G * PLANET_MASS / r0).sqrt(); // ≈ 267 units/s

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
        Planet,
    ));

    // Atmosphere glow ring
    commands.spawn((
        Mesh2d(meshes.add(Annulus::new(PLANET_RADIUS + 1.0, PLANET_RADIUS + 40.0))),
        MeshMaterial2d(materials.add(Color::srgba(0.4, 0.7, 1.0, 0.10))),
        Transform::from_xyz(0., 0., 0.05),
    ));

    // ── Rocket ─────────────────────────────────────────────────────────────

    // Exhaust flame – starts invisible, positioned behind rocket
    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 0.55, 0.05, 0.0),
            custom_size: Some(Vec2::new(7.0, 22.0)),
            ..default()
        },
        Transform::from_xyz(0., r0, 0.9),
        Exhaust,
    ));

    // Rocket body (a slim rectangle; nose = top when angle == 0)
    commands.spawn((
        Sprite {
            color: Color::srgb(0.85, 0.85, 0.95),
            custom_size: Some(Vec2::new(12.0, 28.0)),
            ..default()
        },
        Transform::from_xyz(0., r0, 1.0),
        Rocket {
            velocity: Vec2::new(orbital_v, 0.), // tangent at top → eastward
            angle: 0.0,                          // nose up
            fuel: 100.0,
            throttle: 0.0,
            crashed: false,
        },
    ));

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
        Text::new("A/D: rotate     SPACE / ↑: thrust     R: reset"),
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
    mut q: Query<(&mut Transform, &mut Rocket)>,
) {
    let Ok((mut tf, mut rocket)) = q.get_single_mut() else {
        return;
    };

    // Reset
    if keys.just_pressed(KeyCode::KeyR) {
        let r0 = PLANET_RADIUS + START_HEIGHT;
        let ov = (G * PLANET_MASS / r0).sqrt();
        *tf = Transform::from_xyz(0., r0, 1.0);
        rocket.velocity = Vec2::new(ov, 0.);
        rocket.angle = 0.0;
        rocket.fuel = 100.0;
        rocket.throttle = 0.0;
        rocket.crashed = false;
        return;
    }

    if rocket.crashed {
        return;
    }

    let dt = time.delta_secs();

    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        rocket.angle += ROT_SPEED * dt;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        rocket.angle -= ROT_SPEED * dt;
    }

    let thrusting = keys.pressed(KeyCode::Space)
        || keys.pressed(KeyCode::ArrowUp)
        || keys.pressed(KeyCode::KeyW);
    rocket.throttle = if thrusting && rocket.fuel > 0.0 { 1.0 } else { 0.0 };

    tf.rotation = Quat::from_rotation_z(rocket.angle);
}

fn physics_step(time: Res<Time>, mut q: Query<(&mut Transform, &mut Rocket)>) {
    let Ok((mut tf, mut rocket)) = q.get_single_mut() else {
        return;
    };
    if rocket.crashed {
        return;
    }

    let dt = time.delta_secs();
    let pos = tf.translation.truncate();

    // Gravity: a = −G·M/r² · r̂  (toward planet center at origin)
    let r = pos.length().max(1.0);
    let grav = -pos.normalize() * (G * PLANET_MASS / (r * r));
    rocket.velocity += grav * dt;

    // Thrust: nose direction when angle=0 is +Y
    if rocket.throttle > 0.0 && rocket.fuel > 0.0 {
        let nose = Vec2::new(-rocket.angle.sin(), rocket.angle.cos());
        rocket.velocity += nose * THRUST * dt;
        rocket.fuel = (rocket.fuel - FUEL_RATE * dt).max(0.0);
    }

    // Euler position integration
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

fn update_trajectory(rocket_q: Query<(&Transform, &Rocket)>, mut gizmos: Gizmos) {
    let Ok((rtf, rocket)) = rocket_q.get_single() else {
        return;
    };
    if rocket.crashed {
        return;
    }

    let mu = G * PLANET_MASS;
    let pos = rtf.translation.truncate();
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
            center + rot.rotate(Vec2::new(a * theta.cos(), b * theta.sin()))
        })
        .collect();

    gizmos.linestrip_2d(points, color);
}

fn check_crash(mut q: Query<(&Transform, &mut Rocket)>) {
    let Ok((tf, mut rocket)) = q.get_single_mut() else {
        return;
    };
    if !rocket.crashed && tf.translation.truncate().length() <= PLANET_RADIUS + 2.0 {
        rocket.crashed = true;
        rocket.velocity = Vec2::ZERO;
        rocket.throttle = 0.0;
    }
}

fn update_exhaust(
    rocket_q: Query<(&Transform, &Rocket)>,
    mut exhaust_q: Query<(&mut Transform, &mut Sprite), (With<Exhaust>, Without<Rocket>)>,
) {
    let Ok((rtf, rocket)) = rocket_q.get_single() else {
        return;
    };
    let Ok((mut etf, mut esp)) = exhaust_q.get_single_mut() else {
        return;
    };

    // Tail direction = opposite of nose
    let tail_dir = Vec2::new(rocket.angle.sin(), -rocket.angle.cos());
    let epos = rtf.translation.truncate() + tail_dir * 24.0;
    etf.translation = epos.extend(0.9);
    etf.rotation = rtf.rotation;

    let alpha = if rocket.throttle > 0.0 && rocket.fuel > 0.0 {
        0.9
    } else {
        0.0
    };
    esp.color = Color::srgba(1.0, 0.55, 0.05, alpha);
}

fn follow_camera(
    rocket_q: Query<&Transform, With<Rocket>>,
    mut cam_q: Query<&mut Transform, (With<Camera2d>, Without<Rocket>)>,
    time: Res<Time>,
) {
    let Ok(rtf) = rocket_q.get_single() else {
        return;
    };
    let Ok(mut ctf) = cam_q.get_single_mut() else {
        return;
    };
    let target = Vec3::new(rtf.translation.x, rtf.translation.y, ctf.translation.z);
    ctf.translation = ctf.translation.lerp(target, 6.0 * time.delta_secs());
}

fn update_hud(
    rocket_q: Query<(&Transform, &Rocket)>,
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

    if let Ok(mut t) = alt_q.get_single_mut() {
        *t = Text::new(format!("ALT  {:>8.0} m", alt));
    }
    if let Ok(mut t) = vel_q.get_single_mut() {
        *t = Text::new(format!("VEL  {:>8.1} m/s", speed));
    }
    if let Ok(mut t) = fuel_q.get_single_mut() {
        *t = Text::new(format!("FUEL {:>8.1}", rocket.fuel));
    }
    if let Ok(mut t) = status_q.get_single_mut() {
        *t = Text::new(if rocket.crashed {
            "CRASHED  –  press R to reset"
        } else if rocket.fuel <= 0.0 {
            "OUT OF FUEL"
        } else {
            ""
        });
    }
}
