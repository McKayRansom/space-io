use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    body::CelestialBody, FUEL_RATE, G, LANDING_MAX_SPEED, PLANET_RADIUS, PLANET_SURFACE_GRAVITY,
    ROT_FORCE,
};

const FUEL_TANK_SIZE: Vec2 = Vec2::new(12.0, 16.0);
pub const ENGINE_SIZE: Vec2 = Vec2::new(10.0, 6.0);
const POD_BOTTOM_Y: f32 = -6.0;
const STAGE_GAP: f32 = 0.0;
pub const DEFAULT_FUEL_PER_TANK: f32 = 40.0;
pub const DEFAULT_THRUST: f32 = super::PLANET_SURFACE_GRAVITY * 1.3;

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
pub struct Rocket {
    pub throttle: f32, // 0 or 1
    pub torque: f32,   // -1 or 0 or 1 rotation
    pub crashed: bool,
    pub landed: bool,
    // landed_body: Option<Entity>, // which body we're on (None when flying)
    // body_offset: Vec2,            // surface-normal offset from that body's center
    pub active_stage: Option<Entity>, // currently burning stage
    pub stage_queue: Vec<Entity>,     // remaining stages, front = next to activate
    pub total_mass: f32,
    pub soi_body: Option<Entity>, // which body we're in the SOI of
    pub tail: Option<Entity>,

    // cached-each-frame
    pub stage_thrust: f32,
    pub stage_fuel: f32,
    pub stage_capacity: f32,
    pub stage_active: bool,
}

/// Build a Quat representing a rocket pointing along `direction` (a unit Vec2).
/// The sprite sheet draws the rocket pointing right (+X), so we rotate it to face
/// the desired direction by computing the CCW angle from +X.
// fn quat_from_dir(direction: Vec2) -> Quat {
//     Quat::from_rotation_z(direction.to_angle() - FRAC_PI_2)
// }

// marker component for the player's rocket
// TODO: Move to player.rs
#[derive(Component)]
pub struct PlayerRocket;

#[derive(Component)]
pub struct CommandPod;

// ── Components ───────────────────────────────────────────────────────────
pub struct FuelTank {
    pub(crate) fuel: f32,
    pub(crate) capacity: f32,
}

pub struct Engine {
    thrust: f32, // units/s²
    active: bool,
}

#[derive(Component)]
pub enum RocketPart {
    Engine(Engine),
    FuelTank(FuelTank),
}

impl RocketPart {
    pub fn size(&self) -> Vec2 {
        match self {
            RocketPart::Engine(_engine) => ENGINE_SIZE,
            RocketPart::FuelTank(_fuel_tank) => FUEL_TANK_SIZE,
        }
    }
}

#[derive(Component)]
pub struct Exhaust;

// ── Resources ─────────────────────────────────────────────────────────────────
#[derive(Resource)]
pub struct RocketAssets {
    pub command_pod_sprite: Sprite,
    pub tank_sprite: Sprite,
    pub engine_sprite: Sprite,
    pub exhaust: Sprite,
}

/// Build a single rocket stage with `fuel_count` tanks, an engine, optional
/// decoupler, and an exhaust flame. Transforms start at default — call
/// `relayout_rocket` to position everything correctly.
pub fn build_stage(
    commands: &mut Commands,
    rocket_assets: &RocketAssets,
    // fuel_count: u32,
    fuel_per_tank: f32,
    thrust: f32,
    child: Option<Entity>,
    offset: Vec2,
) -> (Entity, Entity) {
    let mut engine_commands = commands.spawn((
        rocket_assets.engine_sprite.clone(),
        Transform::from_xyz(0.0, -FUEL_TANK_SIZE.y / 2.0, 0.0),
        RocketPart::Engine(Engine { thrust, active: false }),
        Collider::rectangle(12., 2.0),
        Mass(10.0),
    ));
    engine_commands.with_child((
        Exhaust,
        rocket_assets.exhaust.clone(),
        Transform::from_xyz(0.0, -3.0 - 8.0, 0.0),
        AnimationIndices {
            first: SPRITE_EXHAUST_START,
            last: SPRITE_EXHUAST_END,
        },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    if let Some(child) = child {
        engine_commands.add_child(child);
    }

    let engine = engine_commands.id();

    let fuel_tank = commands
        .spawn((
            rocket_assets.tank_sprite.clone(),
            Transform::from_xyz(offset.x, offset.y - FUEL_TANK_SIZE.y / 2.0, 0.0),
            RocketPart::FuelTank(FuelTank {
                fuel: fuel_per_tank,
                capacity: fuel_per_tank,
            }),
            Collider::rectangle(FUEL_TANK_SIZE.x, FUEL_TANK_SIZE.y),
            Mass(10.0),
        ))
        .add_child(engine)
        .id();

    (fuel_tank, engine)
}

/// Spawn the default two-stage rocket.
pub fn spawn_default_rocket(commands: &mut Commands, assets: &RocketAssets, planet: &Entity) {
    let (stage1, tail1) = build_stage(
        commands,
        assets,
        80.0,
        PLANET_SURFACE_GRAVITY * 1.5,
        None,
        Vec2::new(0.0, -ENGINE_SIZE.y / 2.0),
    );
    let (stage2, _tail2) = build_stage(
        commands,
        assets,
        40.0,
        PLANET_SURFACE_GRAVITY * 1.3,
        Some(stage1),
        Vec2::new(0.0, POD_BOTTOM_Y),
    );
    let pod = commands
        .spawn((
            assets.command_pod_sprite.clone(),
            Transform::default(),
            Collider::rectangle(12., 2.0),
            Mass(10.0),
            CommandPod,
        ))
        .add_child(stage2)
        .id();

    commands
        .spawn((
            Transform::from_xyz(0., PLANET_RADIUS * 1.05, 1.0),
            Visibility::default(),
            Rocket {
                active_stage: Some(stage1),
                stage_queue: vec![stage2],
                soi_body: Some(*planet),
                tail: Some(tail1),
                ..Default::default()
            },
            PlayerRocket,
            // avian2d physics
            RigidBody::Dynamic,
            ExternalForce::new(Vec2::ZERO).with_persistence(false),
            ExternalTorque::new(0.0).with_persistence(false),
            Restitution::new(0.4),
            Friction::new(0.9),
        ))
        .add_child(pod);
    // .add_child(stage1)
    // .add_child(stage2);
}

// ── Animation ─────────────────────────────────────────────────────────────────────
#[derive(Component)]
pub struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

pub fn animate_sprite(
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

// loads rocket assets
pub fn rocket_init(
    commands: &mut Commands,
    asset_server: &AssetServer,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    planet: Entity,
) {
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

    // TODO: Move this elsewhere??? (2nd stage init?)
    spawn_default_rocket(commands, &rocket_assets, &planet);

    commands.insert_resource(rocket_assets);
}

pub fn physics_step(
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
    mut part_parent_q: Query<(&mut RocketPart, &Parent)>,
    // mut part_q: Query<&mut RocketPart>,
) {
    for (tf, mass, mut rocket, mut ext_force, mut ext_torque) in rocket_q.iter_mut() {
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

        // find body with the lowest mass that we are in the SOI of
        let (soi_body, body_tf, body) = bodies
            .iter()
            .min_by(|(_e1, t1, b1), (_e2, t2, b2)| {
                let d1 = (t1.translation.truncate() - pos).length();
                let m1 = if d1 < b1.soi.unwrap_or(f32::MAX) {
                    b1.mass
                } else {
                    // not in SOI
                    f32::MAX
                };
                let d2 = (t2.translation.truncate() - pos).length();
                let m2 = if d2 < b2.soi.unwrap_or(f32::MAX) {
                    b2.mass
                } else {
                    // not in SOI
                    f32::MAX
                };
                m1.partial_cmp(&m2).unwrap()
            })
            .unwrap();
        rocket.soi_body = Some(soi_body);

        // Gravity from parent celestial body
        let to_body = body_tf.translation.truncate() - pos;
        let dist_sq = to_body.length_squared();
        if dist_sq < 1.0 {
            continue;
        }
        let dist = dist_sq.sqrt();
        force += (to_body / dist) * (G * body.mass / dist_sq);

        // Thrust — read engine thrust and consume fuel from the active stage's children
        // if rocket.throttle > 0.0 || rocket.stage_active {
        if let Some(tail) = rocket.tail {
            let mut child = tail;
            let mut new_stage_fuel = 0.0;
            let mut new_stage_thrust = 0.0;
            let mut new_stage_capacity = 0.0;
            while let Ok((mut part, parent)) = part_parent_q.get_mut(child) {
                match part.as_mut() {
                    RocketPart::Engine(engine) => {
                        new_stage_thrust += engine.thrust;
                        engine.active = rocket.stage_active;
                    }
                    RocketPart::FuelTank(tank) => {
                        new_stage_fuel += tank.fuel;
                        new_stage_capacity += tank.capacity;
                        if rocket.stage_active {
                            tank.fuel -=
                                (FUEL_RATE * (tank.fuel / rocket.stage_fuel) * dt).max(0.0);
                        }
                    }
                }
                child = parent.get();
            }

            if rocket.stage_active {
                let nose = tf.local_y().truncate();
                force += nose * rocket.stage_thrust;
            }

            // update for next frame
            rocket.stage_fuel = new_stage_fuel;
            rocket.stage_capacity = new_stage_capacity;
            rocket.stage_thrust = new_stage_thrust;
            rocket.stage_active =
                rocket.throttle > 0.0 && rocket.stage_fuel > 0.0 && rocket.stage_thrust > 0.0;
            // }
        }

        ext_force.set_force(force * mass.value());

        let torque: f32 = rocket.torque * ROT_FORCE;

        ext_torque.set_torque(torque);
        // log::dbg
    }
}

/// Handles rocket collisions with the planet (detected by avian2d).
/// The planet has a Collider::circle so avian fires CollisionStarted events when
/// a rocket's circle collider overlaps it. We decide land vs crash here.
pub fn collision_handler(
    mut commands: Commands,
    mut collision_events: EventReader<Collision>,
    planet_q: Query<(Entity, &CelestialBody), With<RigidBody>>,
) {
    for Collision(contacts) in collision_events.read() {
        // TODO: If normal_impusle and tangent_impulse are less than something, switch to landed
        // landed state: Should be marked not active to the physics system, but if an active ship gets close enough, will need to be re-activated

        if contacts.total_normal_impulse < LANDING_MAX_SPEED
            && contacts.total_tangent_impulse < LANDING_MAX_SPEED
        {
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

pub fn update_exhaust(
    mut exhaust_q: Query<(&Exhaust, &Parent, &mut Sprite)>,
    engine_q: Query<&RocketPart, Without<Exhaust>>,
) {
    for (_exhaust, parent, mut sprite) in exhaust_q.iter_mut() {
        match engine_q.get(parent.get()).unwrap() {
            RocketPart::Engine(engine) => {
                if engine.active {
                    sprite.color = Color::default();
                } else {
                    sprite.color = Color::NONE;
                }
            }
            _ => {}
        }
    }
}
