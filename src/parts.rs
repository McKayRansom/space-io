use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use serde::Deserialize;

use crate::AppState;

// ── Part data definitions (loaded from assets/parts.ron) ──────────────────────

#[derive(Deserialize, Clone, Debug)]
pub struct PartDef {
    pub id: String,
    pub name: String,
    pub mass: f32,
    pub sprite_index: usize,
    pub size: (f32, f32),
    pub kind: PartKind,
}

#[derive(Deserialize, Clone, Debug)]
pub enum PartKind {
    CommandPod,
    FuelTank { capacity: f32 },
    Engine { thrust: f32, fuel_rate: f32 },
}

// ── Asset type ────────────────────────────────────────────────────────────────

#[derive(Asset, TypePath, Deserialize, Debug)]
pub struct PartsConfig {
    pub parts: Vec<PartDef>,
}

// ── Asset loader ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct PartsConfigLoader;

impl AssetLoader for PartsConfigLoader {
    type Asset = PartsConfig;
    type Settings = ();
    type Error = ron::error::SpannedError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.unwrap();
        ron::de::from_bytes(&bytes)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

// ── Resources ─────────────────────────────────────────────────────────────────

/// Holds the in-flight asset handle until the file finishes loading.
#[derive(Resource)]
pub struct PartsConfigHandle(pub Handle<PartsConfig>);

/// Populated once `parts.ron` has been fully loaded. Query this to look up
/// part definitions by index or by `id`.
#[derive(Resource)]
pub struct PartsCatalog(pub Vec<PartDef>);

impl PartsCatalog {
    pub fn find(&self, id: &str) -> Option<&PartDef> {
        self.0.iter().find(|p| p.id == id)
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

pub fn load_parts_config(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("parts.ron");
    commands.insert_resource(PartsConfigHandle(handle));
}

/// Converts the loaded `PartsConfig` asset into a `PartsCatalog` resource.
/// Runs every frame until the asset is ready, then removes itself via the handle resource.
pub fn build_parts_catalog(
    mut commands: Commands,
    handle: Res<PartsConfigHandle>,
    assets: Res<Assets<PartsConfig>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if let Some(config) = assets.get(&handle.0) {
        info!("Parts loaded: {} part(s)", config.parts.len());
        commands.insert_resource(PartsCatalog(config.parts.clone()));
        commands.remove_resource::<PartsConfigHandle>();
        next_state.set(AppState::Playing);
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct PartsPlugin;

impl Plugin for PartsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<PartsConfig>()
            .init_asset_loader::<PartsConfigLoader>()
            .add_systems(Startup, load_parts_config)
            .add_systems(
                Update,
                build_parts_catalog.run_if(resource_exists::<PartsConfigHandle>),
            );
    }
}
