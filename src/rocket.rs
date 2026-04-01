use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    body::CelestialBody, FUEL_RATE, G, LANDING_MAX_SPEED, PLANET_RADIUS, PLANET_SURFACE_GRAVITY,
    ROT_FORCE,
};

const FUEL_TANK_SIZE: Vec2 = Vec2::new(0.0, 16.0);
const ENGINE_SIZE: Vec2 = Vec2::new(0.0, 6.0);
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

// ── Stage components ───────────────────────────────────────────────────────────

#[derive(Component)]
pub struct RocketStage;

#[derive(Component)]
pub struct FuelTank {
    pub(crate) fuel: f32,
    pub(crate) capacity: f32,
}

#[derive(Component)]
pub struct Engine {
    thrust: f32, // units/s²
}

// ── Resources ─────────────────────────────────────────────────────────────────
#[derive(Resource)]
pub struct RocketAssets {
    pub command_pod_sprite: Sprite,
    pub tank_sprite: Sprite,
    pub engine_sprite: Sprite,
    pub exhaust: Sprite,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a single rocket stage with `fuel_count` tanks, an engine, optional
/// decoupler, and an exhaust flame. Transforms start at default — call
/// `relayout_rocket` to position everything correctly.
pub fn build_stage(
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
pub fn spawn_default_rocket(commands: &mut Commands, assets: &RocketAssets, planet: &Entity) {
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
                soi_body: Some(*planet),
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
    stage_q: Query<&Children, With<RocketStage>>,
    engine_q: Query<&Engine>,
    mut tank_q: Query<&mut FuelTank>,
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

// ── Rocket layout ─────────────────────────────────────────────────────────────

/// Reposition all stages and their children so they stack neatly under the pod,
/// then shift everything so the entity origin sits at the center of mass.
/// TODO: Remove this!!! and just have each part be offset from it's parent
pub fn relayout_rocket(
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
