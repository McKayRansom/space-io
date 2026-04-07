use avian2d::prelude::mass_properties::components::RecomputeMassProperties;

use super::*;

struct SeparateStage(Entity);

impl Command for SeparateStage {
    fn apply(self, world: &mut World) {
        let (lin_vel, tail) = {
            let mut q = world.query::<(&LinearVelocity, &Rocket)>();
            let (lv, r) = q.get(world, self.0).unwrap();
            (*lv, r.tail)
        };

        let Some(tail_entity) = tail else { return };
        let mut q = world.query::<(&RocketPart, &GlobalTransform, &ChildOf)>();

        let mut current_entity = tail_entity;
        loop {
            let (part_is_decoupler, gt, parent) = {
                let Ok((p, g, c)) = q.get(world, current_entity) else { return };
                (matches!(p, RocketPart::Decoupler), *g, c.parent())
            };

            println!("Traversing {}", current_entity);

            if part_is_decoupler {
                let world_tf = gt.compute_transform();
                let nose = world_tf.local_y().truncate();
                let sep_vel = *lin_vel - STAGE_SEP_VEL * nose;
                let soi_body = world.get::<Rocket>(self.0).unwrap().soi_body;
                let new_rocket = world.spawn((
                    Visibility::default(),
                    world_tf,
                    Rocket { tail: Some(tail_entity), soi_body, ..Default::default() },
                    RigidBody::Dynamic,
                    LinearVelocity(sep_vel),
                    RigidBodyColliders::default(), // pre-seed so on_insert hooks push synchronously
                )).id();

                world.entity_mut(new_rocket).add_child(current_entity);

                let mut tail_cmds = world.entity_mut(current_entity);
                tail_cmds.get_mut::<Transform>().unwrap()
                    .set_if_neq(Transform::default());
                tail_cmds.insert(ColliderOf{body: new_rocket});

                let mut fix_collider_entity = tail_entity;
                while fix_collider_entity != current_entity {
                    let mut ent_mut = world.entity_mut(fix_collider_entity);
                    ent_mut.insert(ColliderOf {body: new_rocket});
                    println!("Fixing: {}", fix_collider_entity);
                    fix_collider_entity = ent_mut.get::<ChildOf>().unwrap().0;
                }

                world.get_mut::<Rocket>(self.0).unwrap().tail = Some(parent);
                world.entity_mut(self.0).insert(RecomputeMassProperties);
                world.entity_mut(new_rocket).insert(RecomputeMassProperties);
                println!("Parent Rocket colliders: {:?}", world.get::<RigidBodyColliders>(self.0).unwrap());
                println!("New Rocket colliders: {:?}", world.get::<RigidBodyColliders>(new_rocket).unwrap());
                return;
            }

            current_entity = parent;
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

        spawn_default_rocket(&mut commands, &planet.0);
        return;
    }

    if rocket.crashed {
        return;
    }

    // Stage separation — find next decoupler and split the rocket there
    if keys.just_pressed(KeyCode::Space) && rocket.tail.is_some() {
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
        commands.queue(rocket::Takeoff{rocket_entity});
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
