use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    body::{CelestialBody, PlanetEntity},
    parts::{PartKind, PartsCatalog, SpritesheetInfo},
    AppState, G, LANDING_MAX_SPEED, PLANET_RADIUS, ROT_FORCE,
};

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct Rocket {
    pub throttle: f32, // 0 or 1
    pub torque: f32,   // -1 or 0 or 1 rotation
    pub crashed: bool,
    pub landed: bool,
    // landed_body: Option<Entity>, // which body we're on (None when flying)
    // body_offset: Vec2,            // surface-normal offset from that body's center
    pub soi_body: Option<Entity>, // which body we're in the SOI of
    pub tail: Option<Entity>,

    // cached-each-frame
    pub stage_thrust: f32,
    pub stage_fuel_rate: f32,
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

// ── Components ───────────────────────────────────────────────────────────
pub struct FuelTank {
    pub(crate) fuel: f32,
    pub(crate) capacity: f32,
}

pub struct Engine {
    thrust: f32, // units/s²
    fuel_rate: f32,
    active: bool,
}

#[derive(Component)]
pub enum RocketPart {
    CommandPod,
    Decoupler,
    Engine(Engine),
    FuelTank(FuelTank),
}

// impl RocketPart {
//     pub fn size(&self) -> Vec2 {
//         match self {
//             RocketPart::Engine(_engine) => ENGINE_SIZE,
//             RocketPart::FuelTank(_fuel_tank) => FUEL_TANK_SIZE,
//         }
//     }
// }

#[derive(Component)]
pub struct Exhaust;

pub struct RocketBuildCommand {
    pub rocket: Entity,
    pub part_id: String,
}

impl Command for RocketBuildCommand {
    fn apply(self, world: &mut World) {
        let catalog = world.get_resource::<PartsCatalog>().unwrap();
        let part_def = catalog.find(&self.part_id).unwrap().clone();
        let sprite = part_def.sprite(world.get_resource::<SpritesheetInfo>().unwrap());

        // TEMP: Assume Tail or Rocket is parent, eventually we will want to specify
        let rocket = world.get::<Rocket>(self.rocket).unwrap();
        let parent = rocket.tail.unwrap_or(self.rocket);

        let transform = if let Some(tail) = rocket.tail {
            let parent_collider = world.get::<Collider>(tail).unwrap();
            let extents = parent_collider.shape().as_cuboid().unwrap().half_extents;
            Transform::from_xyz(0.0, -extents.y - part_def.size.1 / 2.0, 0.0)
        } else {
            Transform::default()
        };

        let part = match part_def.kind {
            PartKind::CommandPod => RocketPart::CommandPod,
            PartKind::Decoupler => RocketPart::Decoupler,
            PartKind::FuelTank { capacity } => RocketPart::FuelTank(FuelTank {
                fuel: capacity,
                capacity,
            }),
            PartKind::Engine { thrust, fuel_rate } => RocketPart::Engine(Engine {
                thrust,
                fuel_rate,
                active: false,
            }),
        };

        let collide = Collider::rectangle(part_def.size.0, part_def.size.1);

        let exhaust = part_def.anim.map(|_anim| {
            let (sprite, anim_indices, anim_timer) =
                part_def.build_anim(world.get_resource::<SpritesheetInfo>().unwrap());
            world
                .spawn((
                    Exhaust,
                    Transform::from_xyz(
                        0.0,
                        -part_def.size.1 / 2.0 - 8.0, /* sprite size compensation */
                        0.0,
                    ),
                    sprite,
                    anim_indices,
                    anim_timer,
                ))
                .id()
        });

        let mut entity = world.spawn((sprite, transform, part, collide, Mass(part_def.mass)));
        if let Some(exhaust) = exhaust {
            entity.add_child(exhaust);
        }

        let id = entity.id();
        let mut parent = world.get_entity_mut(parent).unwrap();
        parent.add_child(id);

        let mut rocket = world.get_mut::<Rocket>(self.rocket).unwrap();
        // assume we're new tail for now
        rocket.tail = Some(id);

        // println!("Spawned rocket part: {}", self.part_id);
    }
}

/// Spawn the default two-stage rocket.
pub fn spawn_default_rocket(commands: &mut Commands, planet: &Entity) {
    let rocket = commands
        .spawn((
            Transform::from_xyz(0., PLANET_RADIUS * 1.10, 1.0),
            Visibility::default(),
            Rocket {
                // active_stage: Some(stage1),
                // stage_queue: vec![stage2],
                soi_body: Some(*planet),
                // tail: Some(tail1),
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
        .id();

    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "command_pod_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "decoupler_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "fuel_tank_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "engine_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "decoupler_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "fuel_tank_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "fuel_tank_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "engine_mk1".into(),
    });
    commands.queue(RocketBuildCommand {
        rocket,
        part_id: "engine_mk1".into(),
    });
}

// ── Animation ─────────────────────────────────────────────────────────────────────
#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

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

pub fn rocket_init(mut commands: Commands, planet_entity: Res<PlanetEntity>) {
    spawn_default_rocket(&mut commands, &planet_entity.0);
}

pub fn physics_step(
    time: Res<Time<Physics>>,
    bodies: Query<(Entity, &Transform, &CelestialBody)>,
    mut rocket_q: Query<
        (
            &Transform,
            &ComputedMass,
            &ComputedCenterOfMass,
            &mut Rocket,
            &mut ExternalForce,
            &mut ExternalTorque,
        ),
        Without<CelestialBody>,
    >,
    mut part_parent_q: Query<(&mut RocketPart, &Parent)>,
    // mut part_q: Query<&mut RocketPart>,
) {
    for (tf, mass, com, mut rocket, mut ext_force, mut ext_torque) in rocket_q.iter_mut() {
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
        let pos =
            tf.translation.truncate() + (tf.rotation * Vec3::new(com.x, com.y, 0.0)).truncate();

        let mut force: Vec2 = Vec2::ZERO;
        let mut lowest_mass: f32 = f32::MAX;
        // let mut soi

        for (entity, body_tf, body) in bodies.iter() {

            // Gravity from parent celestial body
            let to_body = body_tf.translation.truncate() - pos;

            let dist_sq = to_body.length_squared();
            if dist_sq < 1.0 {
                continue;
            }
            let dist = dist_sq.sqrt();

            if dist > body.soi.unwrap_or(f32::MAX) {
                continue;
            }

            if body.mass < lowest_mass {
            
                // println!("SOI BODY: {} mass: {}" , soi_body, body.mass);
                lowest_mass = body.mass;
                rocket.soi_body = Some(entity);
            }

            force += mass.value() * (to_body / dist) * (G * body.mass / dist_sq);
        }

        // Thrust — read engine thrust and consume fuel from the active stage's children
        // if rocket.throttle > 0.0 || rocket.stage_active {
        if let Some(tail) = rocket.tail {
            let mut child = tail;
            let mut new_stage_fuel = 0.0;
            let mut new_stage_thrust = 0.0;
            let mut new_stage_capacity = 0.0;
            let mut new_stage_fuel_rate = 0.0;
            while let Ok((mut part, parent)) = part_parent_q.get_mut(child) {
                match part.as_mut() {
                    RocketPart::Engine(engine) => {
                        new_stage_thrust += engine.thrust;
                        new_stage_fuel_rate += engine.fuel_rate;
                        engine.active = rocket.stage_active;
                    }
                    RocketPart::FuelTank(tank) => {
                        new_stage_fuel += tank.fuel;
                        new_stage_capacity += tank.capacity;
                        if rocket.stage_active {
                            tank.fuel -=
                                (rocket.stage_fuel_rate * (tank.fuel / rocket.stage_fuel) * dt)
                                    .max(0.0);
                        }
                    }
                    RocketPart::Decoupler => {
                        // This is the end of the stage!
                        break;
                    }
                    _ => {}
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
            rocket.stage_fuel_rate = new_stage_fuel_rate;
            rocket.stage_active =
                rocket.throttle > 0.0 && rocket.stage_fuel > 0.0 && rocket.stage_thrust > 0.0;
            // println!("Rocket stuff: {} {} {}", new_stage_fuel, new_stage_thrust, rocket.stage_active);
            // }
        }

        ext_force.set_force(force);

        let torque: f32 = rocket.torque * ROT_FORCE;

        ext_torque.set_torque(torque);
        // log::dbg
    }
}

/// Handles rocket collisions with the planet (detected by avian2d).
/// The planet has a Collider::circle so avian fires CollisionStarted events when
/// a rocket's circle collider overlaps it. We decide land vs crash here.
pub fn collision_handler(
    mut rocket_q: Query<&mut Rocket, Without<CelestialBody>>,
    mut commands: Commands,
    mut collision_events: EventReader<Collision>,
    part_q: Query<&Parent, (With<RocketPart>, Without<Rocket>)>,
) {
    for Collision(contacts) in collision_events.read() {
        // TODO: If normal_impusle and tangent_impulse are less than something, switch to landed
        // landed state: Should be marked not active to the physics system, but if an active ship gets close enough, will need to be re-activated

        if contacts.total_normal_impulse < LANDING_MAX_SPEED
            && contacts.total_tangent_impulse < LANDING_MAX_SPEED
        {
            continue;
        }

        if let Ok(mut rocket) = rocket_q.get_mut(contacts.body_entity1.unwrap()) {
            if rocket.tail.is_some_and(|tail| tail == contacts.entity1) {
                // we gotta move it
                println!("YAY1");
                rocket.tail = Some(part_q.get(contacts.entity1).unwrap().get());
            }
            commands.entity(contacts.entity1).despawn_recursive();
        }

        if let Ok(mut rocket) = rocket_q.get_mut(contacts.body_entity2.unwrap()) {
            if rocket.tail.is_some_and(|tail| tail == contacts.entity2) {
                // we gotta move it
                println!("YAY2");
                rocket.tail = Some(part_q.get(contacts.entity2).unwrap().get());
            }
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

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct RocketPlugin;

impl Plugin for RocketPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing), rocket_init)
            .add_systems(
                // TODO: Do these need to be moved to a physics schedule?
                FixedUpdate,
                (physics_step, collision_handler).run_if(in_state(AppState::Playing)),
            )
            .add_systems(
                Update,
                (update_exhaust, animate_sprite)
                    .run_if(in_state(AppState::Playing)),
            );
    }
}
