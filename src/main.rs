//! Space IO – 2D KSP-style prototype (Bevy 0.15)
//!
//! Controls
//! --------
//!   A / ←      rotate left  (CCW)
//!   D / →      rotate right (CW)
//!   Space/↑/W  main engine  (thrust)
//!   R          reset to orbit

use avian2d::{prelude::*, sync::ancestor_marker::AncestorMarker};
use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,

};
use rand::Rng;

mod body;
use body::*;
mod rocket;
use rocket::*;
mod shaders;
use shaders::*;
mod hud;
use hud::*;
mod player;
use player::*;
mod editor;
use editor::*;

// ── Physics constants  ─────────────────────────
const G: f32 = 20.0;
const PLANET_MASS: f32 = 5.0e6; // G·M = 2·10⁷
const PLANET_RADIUS: f32 = 3400.0;
const PLANET_SURFACE_GRAVITY: f32 = G * PLANET_MASS / (PLANET_RADIUS * PLANET_RADIUS);
const FUEL_RATE: f32 = 1.0; // fuel/s at full throttle (per tank)
const ROT_FORCE: f32 = 3000.0; // no idea on the units on this

const MOON_SURFACE_GRAVITY: f32 = PLANET_SURFACE_GRAVITY / 6.0;
const MOON_MASS: f32 = MOON_SURFACE_GRAVITY * (MOON_RADIUS * MOON_RADIUS) / G; // G·M_moon = 1·10⁶
const MOON_RADIUS: f32 = 200.0;
const MOON_ORBIT: f32 = 10000.0; // distance from planet center

const LANDING_MAX_SPEED: f32 = 200.0; // max speed (or relative speed) for a safe landing
const STAGE_SEP_VEL: f32 = 10.0;

// ── Game constants ────────────────────────────────────────────────────────────────
const DEFAULT_SCALE: f32 = 0.5;
const MAP_VIEW_SCALE: f32 = 25.0;



// ── Resources ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct MapView(bool);


// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(
            PhysicsPlugins::default(), 
        )
        // .add_plugins(PhysicsDebugPlugin::default()) // Enables debug rendering
        .insert_resource(Gravity(Vec2::ZERO)) // we apply custom N-body gravity manually
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Space IO – 2D KSP Prototype".into(),
                        resolution: (1280., 720.).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(ShaderPlugin)
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.08)))
        .insert_resource(MapView::default())
        .add_systems(Startup, (setup, apply_deferred, relayout_rocket).chain())
        .add_systems(
            Update,
            (
                handle_input,
                handle_editor_input,
                update_bodies,
                // physics_step,
                // handle_planet_collision,
                // check_surface_contact,
                // relayout_rocket,
                collision_handler,
                rebuild_editor_ui,
                update_trajectory,
                update_exhaust,
                animate_sprite,
                // follow_camera,
                update_hud,
                update_fps,
            )
                .chain(),
        )
        .add_systems(FixedUpdate, (physics_step, follow_camera))
        // .add_systems(PostUpdate, follow_camera.before(TransformSystem::TransformPropagate))
        .run();
}


// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    mut cloud_materials: ResMut<Assets<CloudMaterial>>,
    mut moon_surface_materials: ResMut<Assets<MoonSurfaceMaterial>>,
    mut moon_crater_materials: ResMut<Assets<MoonCraterMaterial>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let r0 = PLANET_RADIUS; // + START_HEIGHT; // 280 – initial orbit radius
                            // let orbital_v = 0.; //(G * PLANET_MASS / r0).sqrt(); // ≈ 267 units/s

    // assert!(MOON_SURFACE_GRAVITY < PLANET_SURFACE_GRAVITY);

    // Camera – start centered on the rocket
    commands.spawn((Camera2d, Transform::from_xyz(0., r0, 0.)));

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


    #[rustfmt::skip]
    let planet = commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::new(PLANET_RADIUS * 2., PLANET_RADIUS * 2.))),
            MeshMaterial2d(planet_mat(&mut planet_materials)),
            Transform::default(),
            CelestialBody::new(PLANET_MASS, PLANET_RADIUS, 0.0, None, None),
            // avian2d: static body with a circular collider for rocket landing detection
            RigidBody::Static,
            Collider::circle(PLANET_RADIUS),
        ))
        .id();

    #[rustfmt::skip]
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(PLANET_RADIUS * 2., PLANET_RADIUS * 2.))),
        MeshMaterial2d(cloud_mat(&mut cloud_materials)),
        Transform::from_xyz(0., 0., 0.02),
    ));

    // Atmosphere glow ring (visual only, no physics)
    commands.spawn((
        Mesh2d(meshes.add(Annulus::new(PLANET_RADIUS + 1.0, PLANET_RADIUS * 1.1))),
        MeshMaterial2d(materials.add(Color::srgba(0.4, 0.7, 1.0, 0.10))),
        Transform::from_xyz(0., 0., 0.05),
    ));

    let (moon_surface_mat, moon_crater_mat) = moon_mats(&mut moon_surface_materials, &mut moon_crater_materials);
    commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::new(MOON_RADIUS * 2., MOON_RADIUS * 2.))),
            MeshMaterial2d(moon_surface_mat),
            Transform::from_xyz(MOON_ORBIT, 0., 0.2),
            CelestialBody::new(
                MOON_MASS,
                MOON_RADIUS,
                MOON_ORBIT,
                Some(planet),
                Some(PLANET_MASS),
            ),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh2d(meshes.add(Rectangle::new(MOON_RADIUS * 2., MOON_RADIUS * 2.))),
                MeshMaterial2d(moon_crater_mat),
                Transform::from_xyz(0., 0., 0.01),
            ));
        });


    // ── Rocket ─────────────────────────────────────────────────────────────
    rocket_init(&mut commands, &asset_server, &mut texture_atlas_layouts, planet);


    // ── HUD ────────────────────────────────────────────────────────────────
    hud_init(&mut commands);
    
}
