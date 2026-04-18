//! Space IO – 2D KSP-style prototype (Bevy 0.15)
//!
//! Controls
//! --------
//!   A / ←      rotate left  (CCW)
//!   D / →      rotate right (CW)
//!   Space/↑/W  main engine  (thrust)
//!   R          reset to orbit

use avian2d::prelude::*;
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
// use rand::Rng;

mod body;
use body::*;
mod rocket;
use rocket::*;
mod shaders;
use shaders::*;
mod terrain;
use terrain::*;
mod hud;
use hud::*;
mod player;
use player::*;
mod editor;
mod parts;
use parts::*;

use crate::editor::EditorPlugin;

// ── Physics constants  ─────────────────────────
const G: f32 = 10.0;
const PLANET_RADIUS: f32 = 6400.0;
const PLANET_HEIGHT_MAX: f32 = 7000.0;
const PLANET_ATMOSPHERE_MAX: f32 = 8000.0;
const PLANET_SURFACE_GRAVITY: f32 = 10.0;
const PLANET_MASS: f32 = PLANET_SURFACE_GRAVITY * (PLANET_RADIUS * PLANET_RADIUS) / G;
const ROT_FORCE: f32 = 3000.0; // no idea on the units on this

// const MOON_SURFACE_GRAVITY: f32 = PLANET_SURFACE_GRAVITY / 6.0;
const MOON_SURFACE_GRAVITY: f32 = PLANET_SURFACE_GRAVITY /3.0;
const MOON_MASS: f32 = MOON_SURFACE_GRAVITY * (MOON_RADIUS * MOON_RADIUS) / G; // G·M_moon = 1·10⁶
const MOON_RADIUS: f32 = 1000.0;
const MOON_ORBIT: f32 = 20000.0; // distance from planet center

const BREAK_FORCE: f32 = 20000.0; // max force for a safe landing (can be moved to part eventually)
const LANDING_FORCE: f32 = 1000.0; // max force for a safe landing (can be moved to part eventually)
const STAGE_SEP_VEL: f32 = 10.0;

// ── Game constants ────────────────────────────────────────────────────────────────
const DEFAULT_SCALE: f32 = 0.5;
const MAP_VIEW_SCALE: f32 = 35.0;

// ── State ──────────────────────────────────────────────────────────────────────

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    #[default]
    Loading,
    Editing,
    Playing,
}

const STARTING_STATE: AppState = AppState::Editing;

// ── Resources ─────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct MapView(bool);

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    App::new()
        .add_plugins(FrameTimeDiagnosticsPlugin::new(120))
        .add_plugins(PhysicsPlugins::default())
        // .add_plugins(PhysicsDebugPlugin::default()) // Enables debug rendering
        .insert_resource(Gravity(Vec2::ZERO)) // we apply custom N-body gravity manually
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Space IO – 2D KSP Prototype".into(),
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<AppState>()
        .add_plugins((
            ShaderPlugin,
            TerrainPlugin,
            PartsPlugin,
            RocketPlugin,
            PlayerPlugin,
            CelestialBodyPlugin,
            HudPlugin,
            EditorPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.08)))
        .insert_resource(MapView::default())
        // .add_systems(PostUpdate, follow_camera.before(TransformSystem::TransformPropagate))
        .run();
}
