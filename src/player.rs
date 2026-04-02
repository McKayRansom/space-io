use super::*;

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
    mut part_q: Query<
        (&RocketPart, &GlobalTransform, &mut Transform, &Parent),
        (Without<Rocket>, Without<CelestialBody>),
    >,
    bodies: Query<(Entity, &Transform, &CelestialBody), Without<Rocket>>,
    planet: Res<PlanetEntity>,
    moon: Res<MoonEntity>,
) {
    let Ok((rocket_entity, mut tf, mut rocket, mut lin_vel)) = q.get_single_mut() else {
        return;
    };

    // Map view toggle
    if keys.just_pressed(KeyCode::KeyM) {
        map_view.0 = !map_view.0;
    }

    // Reset — despawn current Rocket and spawn new one
    if keys.just_pressed(KeyCode::KeyR) {
        commands.entity(rocket_entity).despawn_recursive();

        spawn_default_rocket(&mut commands, &planet.0);
        return;
    }

    if rocket.crashed {
        return;
    }

    // Stage separation — Create new rocket with this stage
    if keys.just_pressed(KeyCode::Space) && rocket.tail.is_some() {

        let mut tail = rocket.tail.unwrap();
        while let Ok((part, gt, mut tf, parent)) = part_q.get_mut(tail) {
            if matches!(part, RocketPart::Decoupler) {
                // got it
                *tf = Transform::default();
                let nose = tf.local_y().truncate();
                let sep_vel = **lin_vel - STAGE_SEP_VEL * nose;
                commands
                    .spawn((
                        Visibility::default(),
                        gt.clone(),
                        Rocket {
                            // active_stage: Some(current),
                            tail: rocket.tail,
                            soi_body: rocket.soi_body,
                            ..Default::default()
                        },
                        // give the separated stage its own physics body
                        RigidBody::Dynamic,
                        LinearVelocity(sep_vel),
                        // ExternalForce is added automatically!
                        // ExternalForce::new(Vec2::ZERO).with_persistence(false),
                        // Avian's update_collider_parents only processes entities that have
                        // both RigidBody and AncestorMarker<ColliderMarker>. When we reparent
                        // the stage subtree here, the OnAdd<ColliderMarker> observer that
                        // normally adds this marker walks UP the chain but stops at the stage
                        // entity (which already has the marker), so it never reaches this new
                        // root. We insert it manually so the next FixedPostUpdate correctly
                        // re-registers all descendant colliders to this rigid body.
                        // AncestorMarker::<ColliderMarker>::default(),
                    ))
                    // this will remove it from the old rocket automatically, which is nice
                    .add_child(tail);

                rocket.tail = Some(parent.get());
                // don't keep going! That would be bad
                return;
            }

            tail = parent.get();
        }

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
        let (_entity, moon_tf, moon) = bodies.get(moon.0).unwrap();
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

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_input,).run_if(in_state(AppState::Playing)));
    }
}
