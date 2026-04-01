
use bevy::prelude::*;

use crate::G;

#[derive(Component)]
pub struct CelestialBody {
    pub mass: f32,
    pub radius: f32,
    pub velocity: Vec2,
    pub parent: Option<Entity>, // we don't want to parent using Bevy, because we want seprate physics objects, etc...
    pub soi: Option<f32>,       // radisu of the sphere of influence
}

impl CelestialBody {
    pub fn new(
        mass: f32,
        radius: f32,
        orbital_radius: f32,
        parent: Option<Entity>,
        parent_mass: Option<f32>,
    ) -> Self {
        let body = Self {
            mass,
            radius,
            // assume starting at +x for now
            velocity: if let Some(parent_mass) = parent_mass {
                Vec2::new(0., (G * parent_mass / orbital_radius).sqrt())
            } else {
                Vec2::ZERO
            },
            parent,
            // cheat and assume orbital_radius instead of semi-major axis (a)
            soi: parent_mass
                .map(|parent_mass| orbital_radius * (mass / parent_mass).powf(2.0 / 5.0)),
        };
        // println!("Body radius: {} soi: {}", radius, body.soi.unwrap_or(f32::MAX));
        body
    }

    // pub fn gravity_at(&self, dist: Vec2) -> f32 {
    //     let mut dist_sq = dist.length_squared();
    //     if dist_sq < 1.0 {
    //         dist_sq = 1.0;
    //     }
    //     // let dist = dist_sq.sqrt();
    //     G * self.mass / dist_sq
    // }
}

pub fn update_bodies(time: Res<Time>, mut bodies: Query<(Entity, &mut Transform, &mut CelestialBody)>) {
    let dt = time.delta_secs();

    // Snapshot positions/masses so we can borrow mutably below
    let states: Vec<(Entity, Vec2, f32)> = bodies
        .iter()
        .map(|(e, tf, b)| (e, tf.translation.truncate(), b.mass))
        .collect();

    for (_entity, mut tf, mut body) in bodies.iter_mut() {
        if body.parent.is_none() {
            continue;
        }
        let pos = tf.translation.truncate();
        let mut accel = Vec2::ZERO;

        if let Some((_other_entity, other_pos, other_mass)) = states
            .iter()
            .find(|(entity, _, _)| entity == &body.parent.unwrap())
        {
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
