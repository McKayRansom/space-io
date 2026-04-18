#![allow(unused)]

use crate::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

// ── Editor components ─────────────────────────────────────────────────────────

#[derive(Component)]
pub struct EditorRoot;

#[derive(Component)]
pub struct PartButton(PartDef);

#[derive(Component)]
pub struct RocketNameInput;

#[derive(Resource, Default)]
pub struct TextInputState {
    pub value: String,
}

#[derive(Component)]
pub struct SaveButton;

#[derive(Component)]
pub struct LoadButton;

#[derive(Component)]
pub struct LaunchButton;

// ── Save / load ───────────────────────────────────────────────────────────────
fn save_path(name: &str) -> PathBuf {
    PathBuf::from("saves").join(format!("{name}.ron"))
}

pub struct SaveRocketCommand {
    pub rocket: Entity,
    pub name: String,
}

impl Command for SaveRocketCommand {
    fn apply(self, world: &mut World) {
        let save = save_rocket(world, self.rocket);
        let path = save_path(&self.name);
        fs::create_dir_all(path.parent().unwrap()).ok();
        match ron::ser::to_string(&save) {
            Ok(text) => {
                fs::write(&path, text).ok();
                info!("Rocket '{}' saved to {path:?}", self.name);
            }
            Err(e) => warn!("Failed to serialize rocket save: {e}"),
        }
    }
}

pub struct LoadRocketCommand {
    // pub rocket: Entity,
    pub transform: Transform,
    pub name: String,
    pub planet: Option<Entity>,
}

impl Command for LoadRocketCommand {
    fn apply(self, world: &mut World) {
        let path = save_path(&self.name);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to read save {path:?}: {e}");
                return;
            }
        };
        let save: RocketSave = match ron::de::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to parse save: {e}");
                return;
            }
        };

        SpawnRocketCommand {
            transform: self.transform,
            rocket_save: save,
            planet: self.planet,
        }
        .apply(world);

        info!("Rocket '{}' loaded from {path:?}", self.name);
    }
}

pub struct LaunchRocketCommand {
    pub rocket: Entity,
}

impl Command for LaunchRocketCommand {
    fn apply(self, world: &mut World) -> () {
        // save rocket
        let save = save_rocket(world, self.rocket);
        world.insert_resource(PlayerRocketSave(save));

        let mut next_state = world.get_resource_or_init::<NextState<AppState>>();
        next_state.set(AppState::Playing);
    }
}

// ── Rocket editor ─────────────────────────────────────────────────────────────

// Spawn / despawn the editor panel. Rebuilds only when the stage layout changes.
// TODO: Why does this go away when we don't spawn HUD...
pub fn build_editor_ui(mut commands: Commands, parts: Res<PartsCatalog>) {
    println!("FOJFIOEJIS");
    // spawn default rocket (for now)
    commands.queue(SpawnRocketCommand {
        transform: Transform::default(),
        rocket_save: default_rocket(),
        planet: None,
    });
    // editor menu
    commands
        .spawn((
            EditorRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(80.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.85)),
            DespawnOnExit(AppState::Editing),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("Editor Menu"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // Text input: rocket name
            root.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    min_width: Val::Px(170.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
            ))
            .with_children(|input_box| {
                input_box.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    RocketNameInput,
                ));
            });

            root.spawn((
                Text::new("Save"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.5)),
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    min_width: Val::Px(170.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                Interaction::default(),
                SaveButton,
            ));

            root.spawn((
                Text::new("Load"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.5)),
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    min_width: Val::Px(170.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                Interaction::default(),
                LoadButton,
            ));

            root.spawn((
                Text::new("Launch"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.5)),
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    min_width: Val::Px(170.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                Interaction::default(),
                LaunchButton,
            ));
        });

    // parts menu
    commands
        .spawn((
            EditorRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(80.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.85)),
            DespawnOnExit(AppState::Editing),
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("Parts List"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // One button per part for now
            // TODO: Add Sprite
            for part in &parts.0 {
                root.spawn((
                    Text::new(part.name.clone()),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.5)),
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        min_width: Val::Px(170.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                    Interaction::default(),
                    PartButton(part.clone()),
                ));
            }
        });
}

pub fn remove_editor_ui(mut commands: Commands, editor_root: Query<Entity, With<EditorRoot>>) {
    for entity in editor_root.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn handle_editor_input(
    mut commands: Commands,
    mut rocket_q: Query<(Entity, &mut Rocket), With<PlayerRocket>>,
    stage_buttons: Query<(&Interaction, &PartButton)>,
    save_btn: Query<&Interaction, With<SaveButton>>,
    load_btn: Query<&Interaction, With<LoadButton>>,
    launch_btn: Query<&Interaction, With<LaunchButton>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut key_events: MessageReader<KeyboardInput>,
    mut text_state: ResMut<TextInputState>,
    mut name_input_q: Query<&mut Text, With<RocketNameInput>>,
) {
    let Ok((rocket_entity, mut rocket)) = rocket_q.single_mut() else {
        return;
    };

    // println!("BAR");

    // buttons
    if mouse.just_pressed(MouseButton::Left) {
        // Stage buttons — left click = add part
        for (interaction, part_btn) in &stage_buttons {
            if *interaction == Interaction::Pressed {
                commands.queue(RocketBuildCommand {
                    rocket: rocket_entity,
                    part_id: part_btn.0.id.clone(),
                });
            }
        }

        // Save / Load buttons (name field must be non-empty)
        let name = text_state.value.trim().to_string();
        if !name.is_empty() {
            if save_btn.single().is_ok_and(|i| *i == Interaction::Pressed) {
                commands.queue(SaveRocketCommand {
                    rocket: rocket_entity,
                    name: name.clone(),
                });
            }
            if load_btn.single().is_ok_and(|i| *i == Interaction::Pressed) {
                // for now: remove current rocket
                commands.entity(rocket_entity).despawn();
                commands.queue(LoadRocketCommand {
                    transform: Transform::default(),
                    name: name.clone(),
                    planet: None,
                });
            }
        }

        // launch
        if launch_btn
            .single()
            .is_ok_and(|i| *i == Interaction::Pressed)
        {
            commands.queue(LaunchRocketCommand {
                rocket: rocket_entity,
            });
        }
    }

    // Text input — typed characters update rocket name field
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Character(c) => text_state.value.push_str(c.as_str()),
            Key::Backspace => {
                text_state.value.pop();
            }
            _ => {}
        }
    }

    // Keep display in sync
    if let Ok(mut text) = name_input_q.single_mut() {
        **text = text_state.value.clone();
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputState>();
        app.add_systems(OnEnter(AppState::Editing), build_editor_ui);
        app.add_systems(OnExit(AppState::Editing), remove_editor_ui);
        app.add_systems(
            Update,
            (handle_editor_input,).run_if(in_state(AppState::Editing)),
        );
    }
}
