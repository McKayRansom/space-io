use avian2d::prelude::*;
use bevy::{log, prelude::*};

use crate::{
    body::{setup_bodies, CelestialBody, PlanetEntity},
    parts::{PartKind, PartsCatalog, SpritesheetInfo},
    AppState, BREAK_FORCE, G, LANDING_FORCE, PLANET_RADIUS, ROT_FORCE,
};

use serde::{Deserialize, Serialize};

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct Rocket {
    pub throttle: f32, // 0 or 1
    pub torque: f32,   // -1 or 0 or 1 rotation
    pub crashed: bool,
    pub landed: bool,
    pub landed_body: Option<Entity>, // which body we're on (None when flying)
    pub disabled_part: Option<Entity>,
    pub body_offset: Option<Vec3>, // surface-normal offset from that body's center
    pub soi_body: Option<Entity>,  // which body we're in the SOI of
    // pub tail: Option<Entity>,

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

/// Records which part definition created this entity, for save/load.
#[derive(Component)]
pub struct PartId(pub String);

#[derive(Serialize, Deserialize, Clone)]
pub struct RocketSave {
    pub name: String,
    pub parts: Vec<RocketSavePart>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RocketSavePart {
    pub id: String,
    pub tf: Vec2,
    pub pt: usize,
}

// the player's current rocketsave
#[derive(Resource)]
pub struct PlayerRocketSave(pub RocketSave);

pub fn spawn_rocket_part(world: &mut World, id: String, transform: Vec2) -> Entity {
    let catalog = world.get_resource::<PartsCatalog>().unwrap();
    let part_def = catalog.find(&id).unwrap().clone();
    let sprite = part_def.sprite(world.get_resource::<SpritesheetInfo>().unwrap());

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

    let mut entity = world.spawn((
        sprite,
        Transform::from_translation(transform.extend(0.0)),
        part,
        Collider::rectangle(part_def.size.0, part_def.size.1),
        Mass(part_def.mass),
        PartId(id),
    ));
    if let Some(exhaust) = exhaust {
        entity.add_child(exhaust);
    }

    entity.id()
}

pub struct SpawnRocketCommand {
    pub transform: Transform,
    pub rocket_save: RocketSave,
    pub planet: Option<Entity>,
}

pub fn spawn_non_player_rocket(world: &mut World, transform: Transform, child: Entity, rocket: Rocket, velocity: LinearVelocity) -> Entity {
    let state = world.resource::<State<AppState>>().get().clone();

    world
        .spawn((
            transform,
            Visibility::default(),
            rocket,
            RigidBody::Dynamic,
            DespawnOnExit(state),
            velocity,
        ))
        .add_child(child)
        .id()
}

impl Command for SpawnRocketCommand {
    fn apply(self, world: &mut World) -> () {
        let state = world.resource::<State<AppState>>().get().clone();

        let rocket = world
            .spawn((
                self.transform,
                Visibility::default(),
                Rocket {
                    soi_body: self.planet,
                    // tail: Some(tail1),
                    ..Default::default()
                },
                PlayerRocket,
                // avian2d physics
                RigidBody::Dynamic,
                // Restitution::new(0.4),
                // Friction::new(0.9),
                DespawnOnExit(state),
            ))
            .id();

        let mut entity_lookup: Vec<Entity> = Vec::new();
        for (i, part) in self.rocket_save.parts.iter().enumerate() {
            let id = spawn_rocket_part(world, part.id.clone(), part.tf);
            if i > 0 {
                if let Some(parent_id) = entity_lookup.get(part.pt) {
                    world.entity_mut(*parent_id).add_child(id);
                } else {
                    log::warn!(
                        "Unable to find proper parent for part {}, wanted parent {}",
                        part.id,
                        part.pt
                    );
                }
            } else {
                world.entity_mut(rocket).add_child(id);
            }
            entity_lookup.push(id);
        }
    }
}

pub fn rocket_all_parts_world(world: &mut World, rocket: Entity) -> Vec<Entity> {
    let mut child_q = world.query::<&Children>();
    let mut part_q = world.query::<&RocketPart>();

    rocket_all_parts(rocket, child_q.query(world), part_q.query(world))
}

// Really, this just finds all children who are rocket parts, including the rocket itself, but that works for how we use it
pub fn rocket_all_parts(rocket: Entity, child_q: Query<&Children>, part_q: Query<&RocketPart>) -> Vec<Entity> {

    let mut queue: Vec<Entity> = vec![rocket];
    let mut parts: Vec<Entity> = Vec::new();

    while let Some(entity) = queue.pop() {
        println!("Parsing entity: {}", entity);
        if let Ok(children) = child_q.get(entity) {
            for child in children {
                println!("need to check child: {}", child);
                queue.push(*child);
            }
        }
        // skip the rocket entity itself, or any exhausts
        if part_q.get(entity).is_ok() {
            println!("Found rocket part {}", entity);
            parts.push(entity);
        }
    }

    parts
}

pub fn save_rocket(world: &mut World, rocket_entity: Entity) -> RocketSave {

    let mut parts: Vec<RocketSavePart> = Vec::new();
    let mut entity_lookup: Vec<Entity> = Vec::new();

    let mut part_q = world.query::<(&PartId, &Transform, &ChildOf)>();

    for current in rocket_all_parts_world(world, rocket_entity) {
        if let Ok((id, tf, child_of)) = part_q.get(world, current) {
            parts.push(RocketSavePart {
                id: id.0.clone(),
                tf: tf.translation.truncate(),
                pt: entity_lookup
                    .iter()
                    .position(|elem| elem == &child_of.0)
                    .unwrap_or(0),
            });
            entity_lookup.push(current);
        }
    }

    RocketSave {
        name: "Unnamed - TODO".into(),
        parts,
    }
}

pub fn default_rocket() -> RocketSave {
    RocketSave {
        name: "Default Rocket".into(),
        parts: [
            RocketSavePart {
                id: "command_pod_mk1".into(),
                tf: Vec2::default(),
                pt: 0,
            },
            // "decoupler_mk1".into(),
            // "fuel_tank_mk1".into(),
            // "engine_mk1".into(),
            // "decoupler_mk1".into(),
            // "fuel_tank_mk1".into(),
            // "fuel_tank_mk1".into(),
            // "engine_mk1".into(),
            // "engine_mk1".into(),
        ]
        .to_vec(),
    }
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

pub fn rocket_init(
    mut commands: Commands,
    player_rocket: Res<PlayerRocketSave>,
    planet_entity: Res<PlanetEntity>,
) {
    commands.queue(SpawnRocketCommand {
        transform: Transform::from_xyz(0., PLANET_RADIUS * 1.10, 1.0),
        rocket_save: player_rocket.0.clone(),
        planet: Some(planet_entity.0),
    });
}

pub fn physics_step(
    time: Res<Time<Physics>>,
    bodies: Query<(Entity, &Transform, &CelestialBody)>,
    mut rocket_q: Query<
        (Entity, &mut Transform, &ComputedCenterOfMass, &mut Rocket, Forces),
        Without<CelestialBody>,
    >,
    child_q: Query<&Children>,
    mut part_q: Query<&mut RocketPart>,
) {
    for (rocket_entity, mut tf, com, mut rocket, mut forces) in rocket_q.iter_mut() {
        if rocket.crashed || rocket.landed {
            if let Some(landed_body) = rocket.landed_body {
                let (_end, body_tf, _body) = bodies.get(landed_body).unwrap();
                tf.translation = body_tf.translation + rocket.body_offset.unwrap();
            }
            continue;
        }

        let dt = time.delta_secs();
        let pos =
            tf.translation.truncate() + (tf.rotation * Vec3::new(com.x, com.y, 0.0)).truncate();

        let (soi_ent, soi_body_tf, _soi_body) = bodies.get(rocket.soi_body.unwrap()).unwrap();
        let soi_body_pos = soi_body_tf.translation.truncate();
        let mut lowest_mass: f32 = f32::MAX;

        for (entity, body_tf, body) in bodies.iter() {
            // For the SOI body, apply gravity from the rocket's actual position.
            // For all other bodies, apply gravity as if the rocket is at the SOI body's
            // position (patched conics approximation — the parent body accelerates the whole
            // SOI system, so only the differential matters for local motion prediction).
            let to_body_real = body_tf.translation.truncate() - pos;
            let dist_sq_real = to_body_real.length_squared();
            if dist_sq_real < 1.0 {
                continue;
            }
            let dist_real = dist_sq_real.sqrt();

            if dist_real > body.soi.unwrap_or(f32::MAX) {
                continue;
            }

            // For gravity force, use actual position for the SOI body, but the SOI body's
            // position for all others (patched conics: parent gravity cancels in local frame).
            let to_body = if entity == soi_ent {
                to_body_real
            } else {
                body_tf.translation.truncate() - soi_body_pos
            };
            let dist_sq = to_body.length_squared();
            if dist_sq < 1.0 {
                continue;
            }
            let dist = dist_sq.sqrt();

            if body.mass < lowest_mass {
                lowest_mass = body.mass;
                rocket.soi_body = Some(entity);
                // if entity != soi_ent {
                //     log::info!("SOI Changed. Body: {} mass: {}", soi_ent, body.mass);
                // }
            }

            forces.apply_linear_acceleration((to_body / dist) * (G * body.mass / dist_sq));

            if let Some(atmosphere) = &body.atmosphere {
                if dist_real < atmosphere.extent {
                    // calculate atmospheric drag...
                    // TODO: calculate relative velocity
                    // max thickness (1) at surface
                    let atmos_thickness: f32 = ((atmosphere.extent - dist_real)
                        / (atmosphere.extent - body.radius))
                        .max(0.0);
                    let (vel_dir, vel_mag) = forces.linear_velocity().normalize_and_length();
                    forces.apply_force(
                        -vel_dir * (vel_mag * vel_mag) * atmosphere.drag * atmos_thickness,
                    );
                }
            }
        }

        // Thrust — read engine thrust and consume fuel from the active stage's children
        // if rocket.throttle > 0 || rocket.stage_active {
        let mut force: Vec2 = Vec2::ZERO;
        let parts = rocket_all_parts(rocket_entity, child_q, part_q.as_readonly());

        // if let Some(tail) = todo!() {
            // let mut child = tail;
            let mut new_stage_fuel = 0.0;
            let mut new_stage_thrust = 0.0;
            let mut new_stage_capacity = 0.0;
            let mut new_stage_fuel_rate = 0.0;
            // while let Ok((mut part, parent)) = part_parent_q.get_mut(child) {
            for part_entity in parts {
                // let part = part_q.get(part_entity).unwrap();
                match &mut *part_q.get_mut(part_entity).unwrap() {
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
                        // break;
                    }
                    _ => {}
                }
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
        // }

        forces.apply_force(force);

        let torque: f32 = rocket.torque * ROT_FORCE;

        forces.apply_torque(torque);
        // log::dbg
    }
}

struct Land {
    rocket_entity: Entity,
    body: Entity,
    part_entity: Entity,
}

impl Command for Land {
    fn apply(self, world: &mut World) -> () {
        // let rocket_transform = world.get::<GlobalTransform>(self.rocket_entity).unwrap();
        let body_transform = world.get::<Transform>(self.body).unwrap().clone();
        // let new_transform = rocket_transform.reparented_to(body_transform);
        // *world.get_mut::<Transform>(self.rocket_entity).unwrap() = new_transform;

        // world.get_entity_mut(self.body).unwrap().add_child(self.rocket_entity);

        // let mut rocket_cmds = world.entity_mut(self.rocket_entity);

        let mut rocket_cmds = world.entity_mut(self.rocket_entity);

        let rocket_transform = rocket_cmds.get::<Transform>().unwrap();

        let rel_pos = rocket_transform.translation - body_transform.translation;

        let mut rocket = rocket_cmds.get_mut::<Rocket>().unwrap();
        rocket.landed = true;
        rocket.landed_body = Some(self.body);
        rocket.disabled_part = Some(self.part_entity);
        rocket.body_offset = Some(rel_pos);

        // rocket_cmds.insert(RigidBodyDisabled);

        // let mut angular_vel = world.get_mut::<AngularVelocity>(self.rocket_entity).unwrap();
        // angular_vel.0 = 0.0;

        // let mut linear_vel = world.get_mut::<LinearVelocity>(self.rocket_entity).unwrap();
        // linear_vel.0 = Vec2::ZERO;

        // TODO: This causes a panic in avian2d's islanding code if this is called after avian2d decides the body is at rest/creates an island
        // let mut part_cmds = world.entity_mut(self.part_entity);
        // part_cmds.insert(ColliderDisabled);
    }
}

pub struct Takeoff {
    pub rocket_entity: Entity,
}

impl Command for Takeoff {
    fn apply(self, world: &mut World) -> () {
        // NOTE: If planets rotate in the future that will need to be handled here too!

        let mut rocket = world.get_entity_mut(self.rocket_entity).unwrap();
        // let parent = rocket.get::<ChildOf>().unwrap().clone();
        // rocket.remove_parent_in_place();
        rocket.remove::<RigidBodyDisabled>();

        let mut rocket = rocket.get_mut::<Rocket>().unwrap();
        rocket.landed = false;
        let landed_body = rocket.landed_body.unwrap();
        rocket.landed_body = None;

        if let Some(entity) = rocket.disabled_part {
            world.entity_mut(entity).remove::<ColliderDisabled>();
        }

        let planet_vel = world.get::<LinearVelocity>(landed_body).unwrap().clone();
        let mut rocket_vel = world.get_mut::<LinearVelocity>(self.rocket_entity).unwrap();
        *rocket_vel = planet_vel;
    }
}

/// Handles rocket collisions with the planet (detected by avian2d).
/// The planet has a Collider::circle so avian fires CollisionStarted events when
/// a rocket's circle collider overlaps it. We decide land vs crash here.
pub fn collision_handler(
    mut rocket_q: Query<&mut Rocket, Without<CelestialBody>>,
    _body_q: Query<&CelestialBody>,
    mut commands: Commands,
    collisions: Collisions,
    part_q: Query<&ChildOf, (With<RocketPart>, Without<Rocket>)>,
    vel_q: Query<&LinearVelocity>,
    time: Res<Time<Physics>>,
) {
    for contact_pair in collisions.iter() {
        // NOTE: These are difficult to get not to be stale if things are reparented... probably fixed but maybe not
        let body1 = contact_pair.body1.unwrap();
        let body2 = contact_pair.body2.unwrap();

        let (mut rocket, rocket_entity, part_entity, other_entity) =
            if let Ok(rocket) = rocket_q.get_mut(body1) {
                (rocket, body1, contact_pair.collider1, body2)
            } else if let Ok(rocket) = rocket_q.get_mut(body2) {
                (rocket, body2, contact_pair.collider2, body1)
            } else {
                continue;
            };
        // for (rocket_entity, mut rocket) in &mut rocket_q {
        // let mut total_impulse = 0.0;
        // let mut contact_entity: Option<Entity> = None;

        // for contact_pair in collisions.collisions_with(rocket_entity) {
        let total_impulse = contact_pair.total_normal_impulse_magnitude();

        // println!("Impulse total: {}", total_impulse);
        let force_total = total_impulse / time.delta_secs();
        // println!("Force total: {}", force_total);

        if rocket.landed {
            continue;
        }

        // Sanity check: If relative velocity is < 5, do nothing, if it's greater than 10, always break
        let rocket_vel = vel_q.get(rocket_entity).unwrap();
        let other_vel = vel_q.get(other_entity).unwrap();
        let rel_vel = (other_vel.0 - rocket_vel.0).length();

        // if force_total < LANDING_FORCE && rel_vel < 1.0 {
        //     // land rocket...
        //     if body_q.contains(other_entity) {
        //         log::info!("Landing Rocket force is only {} rel vel {}", force_total, rel_vel);
        //         // mark as landed now so other collisions don't screw things up before the Land command goes through
        //         rocket.landed = true;
        //         commands.queue(Land{rocket_entity, body: other_entity, part_entity});
        //     }
        //     continue;
        // }

        if force_total < BREAK_FORCE || rel_vel < 5.0 {
            continue;
        }

        log::info!(
            "Part {} breaking on contact with {} rel_vel {}",
            part_entity,
            other_entity,
            rel_vel
        );

        // TODO: If normal_impusle and tangent_impulse are less than something, switch to landed
        // or maybe rapier's internal islanding could be querried to decide if it thinks we are landed
        // landed state: Should be marked not active to the physics system, but if an active ship gets close enough, will need to be re-activated

        let parent = part_q.get(part_entity).unwrap().parent();
        if parent == rocket_entity {
            // no other parents, despawn
            commands.entity(parent).despawn();
            continue;
        }

        // if rocket.tail.is_some_and(|tail| tail == part_entity) {
        // we gotta move it

        //     rocket.tail = Some(parent);
        // }
        // TODO: create new rocket with our children if needed
        commands.entity(part_entity).despawn();
        // }
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
    mut exhaust_q: Query<(&Exhaust, &ChildOf, &mut Sprite)>,
    engine_q: Query<&RocketPart, Without<Exhaust>>,
) {
    for (_exhaust, parent, mut sprite) in exhaust_q.iter_mut() {
        match engine_q.get(parent.parent()).unwrap() {
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
        app.insert_resource(PlayerRocketSave(default_rocket()))
            .add_systems(OnEnter(AppState::Playing), rocket_init.after(setup_bodies))
            .add_systems(
                // TODO: Do these need to be moved to a physics schedule?
                FixedUpdate,
                (physics_step, collision_handler).run_if(in_state(AppState::Playing)),
            )
            .add_systems(
                Update,
                (update_exhaust, animate_sprite).run_if(in_state(AppState::Playing)),
            );
    }
}
