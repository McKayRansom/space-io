use avian2d::prelude::mass_properties::components::RecomputeMassProperties;

use super::*;

struct SeparateStage(Entity);

impl Command for SeparateStage {
    fn apply(self, world: &mut World) {
        // let Some(tail_entity) = tail else { return };
        let mut q = world.query::<(&mut RocketPart, &GlobalTransform, &PartStage)>();
        let entites = rocket_all_parts_world(world, self.0);

        // let to_stage: Vec<Entity> = Vec::new();

        // TODO: TEMP just find the first decopuler
        for current_entity in entites.iter().rev() {
            let (mut part, gt, _stage) = q.get_mut(world, *current_entity).unwrap();

            println!("Traversing {}", current_entity);

            match &mut *part {
                // TODO: Parachute from command module for now...
                RocketPart::CommandPod => {
                    // TODO
                }
                RocketPart::Engine(engine) => {
                    if !engine.activated {
                        engine.activated = true;
                        return;
                    }
                }
                RocketPart::Decoupler => {
                    println!("Separating at {}", current_entity);
                    let world_tf = gt.compute_transform();
                    let nose = world_tf.local_y().truncate();
                    let lin_vel = world.get::<LinearVelocity>(self.0).unwrap().0;
                    let sep_vel = lin_vel - STAGE_SEP_VEL * nose;
                    let soi_body = world.get::<Rocket>(self.0).unwrap().soi_body;
                    let new_rocket = spawn_non_player_rocket(
                        world,
                        world_tf,
                        *current_entity,
                        Rocket {
                            soi_body,
                            ..Default::default()
                        },
                        LinearVelocity(sep_vel),
                    );

                    let mut tail_cmds = world.entity_mut(*current_entity);
                    tail_cmds
                        .get_mut::<Transform>()
                        .unwrap()
                        .set_if_neq(Transform::default());

                    // NOTE: I fought this bug for forever: ColliderOf specifies the rigid body of a collider, but it
                    // was not getting updated properly, claude couldn't figure out how to fix it, but this works (I came up with this BTW, not claude)
                    // reinserting the collider seems to ACTUALLY fix it
                    for fix_collider_entity in rocket_all_parts_world(world, *current_entity) {
                        let mut ent_mut = world.entity_mut(fix_collider_entity);
                        let collider = ent_mut.take::<Collider>().unwrap();
                        ent_mut.insert(collider);
                        println!("Fixing: {}", fix_collider_entity);
                    }

                    world.entity_mut(self.0).insert(RecomputeMassProperties);
                    world.entity_mut(new_rocket).insert(RecomputeMassProperties);
                    println!(
                        "Parent Rocket colliders: {:?}",
                        world.get::<RigidBodyColliders>(self.0).unwrap()
                    );
                    println!(
                        "New Rocket colliders: {:?}",
                        world.get::<RigidBodyColliders>(new_rocket).unwrap()
                    );
                    return;
                }
                _ => {}
            }
        }
    }
}

pub fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
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
    bodies: Query<(Entity, &Transform, &LinearVelocity, &CelestialBody), Without<Rocket>>,
    planet: Res<PlanetEntity>,
    moon: Res<MoonEntity>,
    player_rocket: Res<PlayerRocketSave>,
    mut time: ResMut<Time<Physics>>,
) {
    let Ok((rocket_entity, mut tf, mut rocket, mut lin_vel)) = q.single_mut() else {
        return;
    };

    // Map view toggle
    if keys.just_pressed(KeyCode::KeyM) {
        map_view.0 = !map_view.0;
    }

    // Reset — despawn current Rocket and spawn new one
    if keys.just_pressed(KeyCode::KeyR) {
        commands.entity(rocket_entity).despawn();

        rocket_init(commands, player_rocket, planet);
        return;
    }

    if rocket.crashed {
        return;
    }

    // Stage separation — find next decoupler and split the rocket there
    // TODO!!!!
    if keys.just_pressed(KeyCode::Space)
    /* && rocket.tail.is_some() */
    {
        commands.queue(SeparateStage(rocket_entity));
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
    rocket.throttle = if thrusting { 1.0 } else { 0.0 };

    if thrusting && rocket.landed {
        commands.queue(rocket::Takeoff { rocket_entity });
    }

    // debugging tools NOTE: Could be moved to commands
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
        let (_entity, moon_tf, moon_vel, _moon) = bodies.get(moon.0).unwrap();
        let r0 = MOON_RADIUS * 1.3;
        let orbital_v = (G * MOON_MASS / r0).sqrt();
        let new_pos = Vec2::new(moon_tf.translation.x, moon_tf.translation.y + r0);
        tf.translation = Vec3::new(new_pos.x, new_pos.y, 1.0);
        lin_vel.0 = moon_vel.0 + Vec2::new(orbital_v, 0.0);
        rocket.landed = false;
        rocket.crashed = false;
    }
    if keys.just_pressed(KeyCode::Comma) {
        // slow down time
        let speed = time.relative_speed();
        if speed > 1.0 {
            time.set_relative_speed(speed * 0.5);
        }
    }
    if keys.just_pressed(KeyCode::Period) {
        // speed up time
        let speed = time.relative_speed();
        if speed < 8.0 {
            time.set_relative_speed(speed * 2.0);
        }
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_input,).run_if(in_state(AppState::Playing)));
    }
}
