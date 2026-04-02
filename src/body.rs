use bevy::prelude::*;

use crate::{terrain::*, *};

#[derive(Resource)]
pub struct PlanetEntity(pub Entity);

#[derive(Resource)]
pub struct MoonEntity(pub Entity);

#[derive(Component)]
pub struct CelestialBody {
    pub mass: f32,
    pub _radius: f32,
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
            _radius: radius,
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

fn setup_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    shader_mats: Res<CelestialMaterials>,
) {
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

    // ── Planet ─────────────────────────────────────────────────────────────────
    // The planet entity holds physics + the orbit-view mesh (hidden by default).
    // A separate terrain mesh entity holds the close-up view (visible by default).
    // MapView(false) is the default, so terrain meshes start visible.
    let planet_mesh = meshes.add(Rectangle::new(PLANET_RADIUS * 2.0 / 0.8, PLANET_RADIUS * 2.0 / 0.8));

    #[rustfmt::skip]
    let planet = commands
        .spawn((
            Mesh2d(planet_mesh.clone()),
            MeshMaterial2d(shader_mats.planet.clone()),
            OrbitViewMesh,
            Visibility::Hidden, // hidden in default terrain view
            Transform::default(),
            CelestialBody::new(PLANET_MASS, PLANET_RADIUS, 0.0, None, None),
            // avian2d: circle collider stays as backstop; terrain polyline is primary
            RigidBody::Static,
            Collider::circle(PLANET_RADIUS),
            BodyTerrainParams {
                base_radius: PLANET_RADIUS,
                height_scale: (PLANET_RADIUS / 0.8) * 0.2,
                heights: shader_mats.planet_heights.clone(),
            },
        ))
        .id();
    commands.insert_resource(PlanetEntity(planet));

    // Planet terrain mesh (close-up view, visible by default)
    commands.spawn((
        Mesh2d(planet_mesh),
        MeshMaterial2d(shader_mats.planet_terrain.clone()),
        TerrainViewMesh,
        Visibility::Visible,
        Transform::default(),
    ));

    // Cloud layer — orbit view only
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(PLANET_RADIUS * 2., PLANET_RADIUS * 2.))),
        MeshMaterial2d(shader_mats.cloud.clone()),
        OrbitViewMesh,
        Visibility::Hidden,
        Transform::from_xyz(0., 0., 0.02),
    ));

    // Atmosphere glow ring — orbit view only (visual, no physics)
    commands.spawn((
        Mesh2d(meshes.add(Annulus::new(PLANET_RADIUS + 1.0, PLANET_RADIUS * 1.1))),
        MeshMaterial2d(materials.add(Color::srgba(0.4, 0.7, 1.0, 0.10))),
        OrbitViewMesh,
        Visibility::Hidden,
        Transform::from_xyz(0., 0., 0.05),
    ));

    // ── Moon ───────────────────────────────────────────────────────────────────
    let moon_mesh = meshes.add(Rectangle::new(MOON_RADIUS * 2., MOON_RADIUS * 2.));

    let moon = commands
        .spawn((
            Mesh2d(moon_mesh.clone()),
            MeshMaterial2d(shader_mats.moon_surface.clone()),
            OrbitViewMesh,
            Visibility::Hidden,
            Transform::from_xyz(MOON_ORBIT, 0., 0.2),
            CelestialBody::new(
                MOON_MASS,
                MOON_RADIUS,
                MOON_ORBIT,
                Some(planet),
                Some(PLANET_MASS),
            ),
            BodyTerrainParams {
                base_radius: MOON_RADIUS,
                height_scale: MOON_RADIUS * 0.25,
                heights: shader_mats.moon_heights.clone(),
            },
        ))
        .with_children(|parent| {
            // Moon crater layer — orbit view only
            parent.spawn((
                Mesh2d(meshes.add(Rectangle::new(MOON_RADIUS * 2., MOON_RADIUS * 2.))),
                MeshMaterial2d(shader_mats.moon_crater.clone()),
                OrbitViewMesh,
                Visibility::Hidden,
                Transform::from_xyz(0., 0., 0.01),
            ));
            // Moon terrain mesh — close-up view
            parent.spawn((
                Mesh2d(moon_mesh.clone()),
                MeshMaterial2d(shader_mats.moon_terrain.clone()),
                TerrainViewMesh,
                Visibility::Visible,
                Transform::from_xyz(0., 0., 0.0),
            ));
        })
        .id();

    commands.insert_resource(MoonEntity(moon));
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

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct CelestialBodyPlugin;

impl Plugin for CelestialBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_bodies);
        app.add_systems(
            FixedUpdate,
            (update_bodies,).run_if(in_state(AppState::Playing)),
        );
    }
}
