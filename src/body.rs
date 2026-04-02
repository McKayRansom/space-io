use bevy::prelude::*;

use crate::{terrain::*, *};

#[derive(Resource)]
pub struct PlanetEntity(pub Entity);

#[derive(Resource)]
pub struct MoonEntity(pub Entity);

#[derive(Component)]
pub struct CelestialBody {
    pub mass: f32,
    pub radius: f32,
    pub velocity: Vec2,
    pub parent: Option<Entity>, // we don't want to parent using Bevy, because we want seprate physics objects, etc...
    pub soi: Option<f32>,       // radisu of the sphere of influence
    orbital_radius: f32,
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
            orbital_radius,
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

pub struct BodyShaderParams {
    map_shader: PlanetShaders,
    terrain: PlanetShaders,
    map_shader_2: Option<PlanetShaders>,
}

pub fn spawn_body(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    shader_mats: &CelestialMaterials,
    body: CelestialBody,
    terrain_params: BodyTerrainParams,
    shader_params: BodyShaderParams,
) -> Entity {
    let planet_mesh = meshes.add(Rectangle::new(body.radius * 2.0, body.radius * 2.0));
    // planet terrain mesh is slightly bigger...
    let planet_terrain_mesh = meshes.add(Rectangle::new(
        (body.radius + terrain_params.height_scale) * 2.0,
        (body.radius + terrain_params.height_scale) * 2.0,
    ));

    let planet_terrain = shader_mats.spawn_shader(
        commands,
        planet_terrain_mesh,
        shader_params.terrain,
        Some(terrain_params),
    );
    let planet_map_view = shader_mats.spawn_shader(
        commands,
        planet_mesh.clone(),
        shader_params.map_shader,
        None,
    );

    let planet_map_view_2 = shader_params
        .map_shader_2
        .map(|shader| shader_mats.spawn_shader(commands, planet_mesh, shader, None));

    let mut cmds = commands.spawn((
        Visibility::default(),
        Transform::from_xyz(body.orbital_radius, 0.0, 0.0),
        Collider::circle(body.radius),
        body,
        // avian2d: circle collider stays as backstop; terrain polyline is primary
        RigidBody::Static,
    ));

    cmds.add_children(&[planet_terrain, planet_map_view]);

    if let Some(child) = planet_map_view_2 {
        cmds.add_child(child);
    }

    cmds.id()

    // Planet terrain mesh (close-up view, visible by default)

    // Cloud layer — orbit view only
    // commands.spawn((
    //     Mesh2d(meshes.add(Rectangle::new(PLANET_RADIUS * 2., PLANET_RADIUS * 2.))),
    //     MeshMaterial2d(shader_mats.cloud.clone()),
    //     OrbitViewMesh,
    //     Visibility::Hidden,
    //     Transform::from_xyz(0., 0., 0.02),
    // ));

    // Atmosphere glow ring — orbit view only (visual, no physics)
    // commands.spawn((
    //     Mesh2d(meshes.add(Annulus::new(PLANET_RADIUS + 1.0, PLANET_RADIUS * 1.1))),
    //     MeshMaterial2d(materials.add(Color::srgba(0.4, 0.7, 1.0, 0.10))),
    //     OrbitViewMesh,
    //     Visibility::Hidden,
    //     Transform::from_xyz(0., 0., 0.05),
    // ));
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
    let heights = shader_mats.planet_heights.clone(); // TODO: Why is this here, can we generate this within below?
    let planet = spawn_body(
        &mut commands,
        &mut meshes,
        &shader_mats,
        CelestialBody::new(PLANET_MASS, PLANET_RADIUS, 0.0, None, None),
        BodyTerrainParams {
            base_radius: PLANET_RADIUS,
            height_scale: (PLANET_RADIUS / 0.8) * 0.2,
            heights: heights,
        },
        BodyShaderParams {
            map_shader: PlanetShaders::Planet,
            map_shader_2: Some(PlanetShaders::Cloud),
            terrain: PlanetShaders::Terrain,
        },
    );
    commands.insert_resource(PlanetEntity(planet));

    // ── Moon ───────────────────────────────────────────────────────────────────
    let heights = shader_mats.moon_heights.clone(); // TODO: Why is this here, can we generate this within below?
    let moon = spawn_body(
        &mut commands,
        &mut meshes,
        &shader_mats,
        CelestialBody::new(MOON_MASS, MOON_RADIUS, MOON_ORBIT, Some(planet), Some(PLANET_MASS)),
        BodyTerrainParams {
            base_radius: MOON_RADIUS,
            height_scale: (MOON_RADIUS / 0.8) * 0.2,
            heights: heights,
        },
        BodyShaderParams {
            map_shader: PlanetShaders::MoonSurface,
            map_shader_2: Some(PlanetShaders::MoonCrater),
            terrain: PlanetShaders::MoonTerrain,
        },
    );
    commands.insert_resource(MoonEntity(moon));
}

pub fn update_bodies(
    time: Res<Time>,
    mut bodies: Query<(Entity, &mut Transform, &mut CelestialBody)>,
) {
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
