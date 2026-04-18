use std::f32::consts::{PI, TAU};

use avian2d::{
    math::FRAC_PI_2,
    prelude::{ComputedCenterOfMass, LinearVelocity},
};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    text::TextLayoutInfo,
};

use crate::{
    body::CelestialBody,
    rocket::{PlayerRocket, Rocket},
    AppState, MapView, DEFAULT_SCALE, G, MAP_VIEW_SCALE, PLANET_RADIUS,
};

// HUD label — one enum covers all telemetry rows; add a variant to add a new row
#[derive(Component, PartialEq, Eq)]
pub enum HudLabel {
    Alt,
    Vel,
    Fuel,
    Status,
    Nav,
    NavVel,
}
#[derive(Component)]
pub struct HudFps;

const NAV_SIZE: usize = 32;
const NAV_FONT_WIDTH: f32 = 7.5;

const BLUE: Color = Color::srgb(0.55, 0.8, 1.0);
const GREEN: Color = Color::srgb(0.55, 1.0, 0.55);
const YELLOW: Color = Color::srgb(1.0, 0.82, 0.4);
const RED: Color = Color::srgb(1.0, 0.35, 0.35);

pub fn hud_init(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera – start centered on the rocket
    commands.spawn((Camera2d, Transform::from_xyz(0., PLANET_RADIUS, 0.)));

    let mono = TextFont {
        font: asset_server.load("VT323-Regular.ttf"),
        font_size: 18.0,
        ..default()
    };

    commands.spawn((
        Text::new("ALT  --------"),
        mono.clone(),
        TextColor(BLUE),
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
        TextColor(GREEN),
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
        TextColor(YELLOW),
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
        TextColor(RED),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(82.),
            left: Val::Px(12.),
            ..default()
        },
        HudLabel::Status,
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(30.),
            left: Val::Percent(50.0),
            right: Val::Percent(50.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("[{}]", " ".repeat(NAV_SIZE))),
                mono.clone(),
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("U"),
                mono.clone(),
                TextColor(Color::WHITE),
                HudLabel::Nav,
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("+"),
                mono.clone(),
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("o"),
                mono.clone(),
                TextColor(GREEN),
                HudLabel::NavVel,
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));
        });
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
        Text::new("A/D: rotate  W/[UP]: thrust  [SPACE]: stage   M: map   R: reset"),
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

pub fn update_fps(diagnostics: Res<DiagnosticsStore>, mut q: Query<&mut Text, With<HudFps>>) {
    let Ok(mut text) = q.single_mut() else {
        return;
    };
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        *text = Text::new(format!("{fps:.0} FPS"));
    }
}

pub struct OrbitalParameters {
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

pub fn draw_orbit(gizmos: &mut Gizmos, focus: Vec2, orbit: OrbitalParameters) {
    let periapsis = orbit.periapsis();
    let color = if periapsis <= PLANET_RADIUS {
        Color::srgba(1.0, 0.4, 0.2, 0.6) // impact orbit
    } else {
        Color::srgba(0.4, 0.9, 1.0, 0.55) // safe orbit
    };

    // Sample the ellipse and draw as a closed line strip
    const SEGMENTS: usize = 128;
    let points: Vec<Vec3> = (0..=SEGMENTS)
        .map(|i| {
            let theta = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            (focus
                + orbit.center
                + orbit
                    .rot
                    .rotate(Vec2::new(orbit.a * theta.cos(), orbit.b * theta.sin()))
        ).extend(1.0) // draw behind stuff
        })
        .collect();

    gizmos.linestrip(points, color);
}

pub fn update_trajectory(
    rocket_q: Query<
        (&Transform, &LinearVelocity, &Rocket, &ComputedCenterOfMass),
        With<PlayerRocket>,
    >,
    bodies: Query<(&Transform, &LinearVelocity, &CelestialBody)>,
    mut gizmos: Gizmos,
    map_view: Res<MapView>,
) {
    if !map_view.0 {
        return;
    }
    // draw moon trajectory
    for (bt, vel, body) in bodies.iter() {
        if body.parent.is_none() {
            continue;
        }
        let (bt2, _vel2, body2) = bodies.get(body.parent.unwrap()).unwrap();

        let pos = bt.translation.truncate() - bt2.translation.truncate(); // position relative to orbital focus

        if let Some(orbit) = OrbitalParameters::calc(body2.mass, pos, vel.0) {
            draw_orbit(&mut gizmos, bt2.translation.truncate(), orbit);
        }
    }

    let Ok((rtf, rocket_vel, rocket, com)) = rocket_q.single() else {
        return;
    };
    if rocket.crashed || rocket.landed || rocket.soi_body.is_none() {
        return;
    }

    let (body_tf, body_vel, body) = bodies.get(rocket.soi_body.unwrap()).unwrap();
    let focus = body_tf.translation.truncate();

    // position relative to orbital focus
    let pos = rtf.translation.truncate() - focus
        + (rtf.rotation * Vec3::new(com.x, com.y, 0.0)).truncate();
    let relative_vel = **rocket_vel - body_vel.0;

    if let Some(orbit) = OrbitalParameters::calc(body.mass, pos, relative_vel) {
        draw_orbit(&mut gizmos, focus, orbit);
    }
}

pub fn follow_camera(
    rocket_q: Query<(&Transform, &ComputedCenterOfMass), (With<Rocket>, With<PlayerRocket>)>,
    mut cam_q: Query<&mut Transform, (With<Camera2d>, Without<Rocket>)>,
    mut proj_q: Query<&mut Projection, With<Camera2d>>,
    map_view: Res<MapView>,
    time: Res<Time>,
) {
    let Ok((rtf, com)) = rocket_q.single() else {
        return;
    };
    let Ok(mut ctf) = cam_q.single_mut() else {
        return;
    };
    let Ok(mut projection) = proj_q.single_mut() else {
        return;
    };
    let Projection::Orthographic(ref mut proj) = *projection else {
        return;
    };

    let dt = time.delta_secs();
    let rotated_com = rtf.rotation * Vec3::new(com.x, com.y, 0.0);
    let target_pos = Vec3::new(
        rtf.translation.x + rotated_com.x,
        rtf.translation.y + rotated_com.y,
        ctf.translation.z,
    );
    let target_scale = if map_view.0 {
        MAP_VIEW_SCALE
    } else {
        DEFAULT_SCALE
    };

    ctf.translation = ctf.translation.lerp(target_pos, 6.0 * dt);
    proj.scale += (target_scale - proj.scale) * (6.0 * dt).min(1.0);
}

// calculates the margin in pixels of Nav-bar items given the angle in radians
fn nav_calc_margin(angle: f32, pro: &'static str, retro: &'static str) -> (f32, &'static str) {
    let (mark, rel_angle) = if angle.abs() < FRAC_PI_2 {
        // we are pointing "UP"
        (pro, -angle)
    } else {
        // we are pointing "DOWN"
        (
            retro,
            if angle > FRAC_PI_2 {
                -angle + PI
            } else {
                -angle - PI
            },
        )
    };

    let left_margin = ((rel_angle / PI) * NAV_SIZE as f32) * NAV_FONT_WIDTH;
    (left_margin - NAV_FONT_WIDTH / 2.0, mark)
}

pub fn update_hud(
    rocket_q: Query<(&Transform, &LinearVelocity, &Rocket), With<PlayerRocket>>,
    planet_q: Query<(&Transform, &LinearVelocity), With<CelestialBody>>,
    mut hud_q: Query<(&HudLabel, &mut Text, &mut Node, Option<&TextLayoutInfo>)>,
) {
    let Ok((tf, velocity, rocket)) = rocket_q.single() else {
        return;
    };
    let Ok((body_tf, body_vel)) = planet_q.get(rocket.soi_body.unwrap()) else {
        return;
    };
    let alt = (tf.translation.truncate().length() - PLANET_RADIUS).max(0.0);
    let speed = velocity.length();

    for (label, mut text, mut node, layout_info) in &mut hud_q {
        *text = Text::new(match label {
            HudLabel::Alt => format!("ALT  {:>8.0} m", alt),
            HudLabel::Vel => format!("VEL  {:>8.1} m/s", speed),
            HudLabel::Fuel => {
                const W: usize = 10;
                let filled =
                    ((rocket.stage_fuel / rocket.stage_capacity) * W as f32).round() as usize;
                let filled = filled.min(W);
                format!("FUEL [{}{}]", "=".repeat(filled), " ".repeat(W - filled))
            }
            HudLabel::Status => {
                if rocket.crashed {
                    "CRASHED  –  press R to reset".to_string()
                } else if rocket.landed {
                    "LANDED  –  W/↑ to launch\nfoobar".to_string()
                } else if rocket.stage_fuel <= 0.0 {
                    "OUT OF FUEL  –  SPACE to stage".to_string()
                } else {
                    String::new()
                }
            }
            HudLabel::Nav => {
                let radial = (tf.translation.truncate() - body_tf.translation.truncate())
                    .normalize_or_zero();
                let nose = (tf.rotation * Vec3::Y).truncate();
                let angle = -radial.angle_to(nose);
                let (margin, mark) = nav_calc_margin(angle, "U", "D");
                node.left = Val::Px(margin);
                mark.to_string()
            }
            HudLabel::NavVel => {
                let nose = (tf.rotation * Vec3::Y).truncate();
                let rel_vel = velocity.0 - body_vel.0;
                let angle = -rel_vel.angle_to(nose);
                let (margin, mark) = nav_calc_margin(angle, "o", "x");
                node.left = Val::Px(margin);
                mark.to_string()
            }
        });
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, hud_init);
        app.add_systems(
            Update,
            (update_trajectory, update_hud, update_fps)
                .run_if(in_state(AppState::Playing)),
        );
        app.add_systems(
            FixedUpdate,
            (follow_camera,),
            // (follow_camera,).run_if(in_state(AppState::Playing)),
        );
    }
}
