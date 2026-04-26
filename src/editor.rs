#![allow(unused)]

use crate::*;
use avian2d::math::Dir;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::{log, prelude::*};
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

struct SpawnRocketPart {
    id: String,
}

impl Command for SpawnRocketPart {
    fn apply(self, world: &mut World) -> () {
        if let MouseOverState::Selected(old_entity) = world.resource::<MouseOverState>() {
            world.despawn(*old_entity);
        }
        let id = spawn_rocket_part(world, self.id.clone(), Vec2::ZERO, 0);
        world.entity_mut(id).insert(ColliderDisabled);
        world.insert_resource(MouseOverState::Selected(id));
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
                commands.queue(SpawnRocketPart {
                    id: part_btn.0.id.clone(),
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

fn cursor_world_pos(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let (camera, cam_gtf) = camera_q.single().ok()?;
    let window = windows.single().ok()?;
    let screen_pos = window.cursor_position()?; // pixels from top-left
    camera.viewport_to_world_2d(cam_gtf, screen_pos).ok()
}

// fn disable_colliders

struct SelectPart {
    pub id: Entity,
}

impl Command for SelectPart {
    fn apply(self, world: &mut World) -> () {
        let Some(parent) = world
            .get::<ChildOf>(self.id)
            .map(|child_of| child_of.parent())
        else {
            log::warn!("No parent for part {}", self.id);
            return;
        };
        let mut orphan_rocket = None;
        if let Some(_rocket) = world.get::<Rocket>(parent) {
            // this is already the "root" of a rocket, we can just move this
            if let Some(_player_rocket) = world.get::<PlayerRocket>(parent) {
                log::info!("NOT ALLOWED TO MOVE PLAYER ROCKET");
                return;
            }
            log::debug!("Moving whole rocket");
            world.entity_mut(parent).remove::<Children>();
            orphan_rocket = Some(parent);
        }

        // this part is somewhere else on the rocket, unparent this part
        log::debug!("splitting rocket at {}", self.id);
        world.entity_mut(self.id).remove::<ChildOf>();

        // disable colliders, or we're going to have a MAJOR problem
        for entity in rocket_all_parts_world(world, self.id) {
            world.entity_mut(entity).insert(ColliderDisabled);
        }

        world.insert_resource(MouseOverState::Selected(self.id));

        // TODO: this doesn't work, it just despawns the part too even though we removed it as a child, ColliderOf reasons maybe?
        // if let Some(e) = orphan_rocket {
        //     world.despawn(e);
        // }
    }
}

struct DeselectPart {
    pub parent: Option<Entity>,
    pub id: Entity,
}

impl Command for DeselectPart {
    fn apply(self, world: &mut World) -> () {
        if let Some(parent) = self.parent {
            log::debug!("Deslect reparenting {} to {}", self.id, parent);
            assert_ne!(parent, self.id);
            // fix the transform
            let parent_transform = world.get::<GlobalTransform>(parent).unwrap();
            let child_transform = world.get::<GlobalTransform>(self.id).unwrap();
            let new_transform = child_transform.reparented_to(parent_transform);
            // we are placing it on a parent
            world.entity_mut(parent).add_child(self.id);
            world.entity_mut(self.id).insert(new_transform);
        } else {
            // guess we have to spawn a new rocket :(
            let mut transform = world.get_mut::<Transform>(self.id).unwrap();
            let new_tf = transform.clone();
            *transform = Transform::default();
            let new_rocket = spawn_non_player_rocket(
                world,
                new_tf,
                self.id,
                Rocket::default(),
                LinearVelocity(Vec2::ZERO),
            );
            log::warn!("Deselect spawn new rocket {}", new_rocket);
        }

        // enable colliders, or we're going to have a problem
        for entity in rocket_all_parts_world(world, self.id) {
            world.entity_mut(entity).remove::<ColliderDisabled>();
            log::debug!("Enabling collider for {}", entity);
        }
    }
}

#[derive(Resource, Default)]
enum MouseOverState {
    #[default]
    Nothing,
    Hovering(Entity),
    Selected(Entity),
}

const HOVERED_COLOR: Color = Color::srgba(0.0, 1.0, 0.0, 0.6);

// Runs when Nothing or Hovering: find what's under the cursor and update highlight.
fn system_hover(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut part_q: Query<&mut Sprite>,
    mut mouse_over: ResMut<MouseOverState>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    // Clear the previous highlight.
    if let MouseOverState::Hovering(old) = *mouse_over {
        if let Ok(mut sprite) = part_q.get_mut(old) {
            sprite.color = Color::WHITE;
        }
    }

    let Some(world_pos) = cursor_world_pos(windows, camera_q) else {
        *mouse_over = MouseOverState::Nothing;
        return;
    };

    let hits = spatial_query.shape_intersections(
        &Collider::circle(0.5),
        world_pos,
        0.0,
        &SpatialQueryFilter::default(),
    );

    let Some(&entity) = hits.first() else {
        *mouse_over = MouseOverState::Nothing;
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        log::debug!("Selecting entity {}", entity);
        commands.queue(SelectPart { id: entity });
        // NOTE: We cannot change the MouseOverState here for DEEPLY CURSED REASONS, the command will not be processed until after the system below runs and terrible things happen!
    } else {
        if let Ok(mut sprite) = part_q.get_mut(entity) {
            sprite.color = HOVERED_COLOR;
        }
        *mouse_over = MouseOverState::Hovering(entity);
    }
}

// Runs when Selected: move the part with the cursor and snap it on click.
fn system_drag(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut part_q: Query<(&mut Sprite, &mut Transform, &Collider)>,
    mut mouse_over: ResMut<MouseOverState>,
    mouse: Res<ButtonInput<MouseButton>>,
    global_tf: Query<&GlobalTransform>,
) {
    let MouseOverState::Selected(entity) = *mouse_over else {
        return;
    };

    let Ok((mut sprite, mut tf, collider)) = part_q.get_mut(entity) else {
        return;
    };

    let Some(world_pos) = cursor_world_pos(windows, camera_q) else {
        return;
    };
    let world_pos = world_pos.round();

    let hits =
        spatial_query.shape_intersections(collider, world_pos, 0.0, &SpatialQueryFilter::default());

    let mut new_part_pos = world_pos;
    let mut new_part_parent: Option<Entity> = None;

    if let Some(&hit_entity) = hits.first() {
        // Cursor is inside another part — find the nearest free adjacent position.
        // NOTE: Dir::new() can fail on a zero vector.
        let parent_tf = global_tf.get(hit_entity).unwrap();
        let mut pos_diff = parent_tf.translation().truncate() - world_pos;

        // Let's try snapping a bit
        if pos_diff.x.abs() < 5.0 {
            pos_diff.x = 0.0;
        }
        if pos_diff.y.abs() < 5.0 {
            pos_diff.y = 0.0;
        }

        if let Ok(dir) = Dir::new(pos_diff) {
            let start_pos = parent_tf.translation().truncate() - dir * 50.0;
            if let Some(first_hit) = spatial_query.cast_shape(
                collider,
                start_pos,
                0.0,
                dir,
                &ShapeCastConfig::from_max_distance(100.0),
                &SpatialQueryFilter::default(),
            ) {
                new_part_pos = (start_pos + (dir * first_hit.distance)).round();
                new_part_parent = Some(first_hit.entity);
                if first_hit.entity == entity {
                    log::warn!("Collided with selected entity, there must be a problem!");
                    return;
                }
            } else {
                println!("Dir {} Warning: no collision found :(", dir);
            }
        }
    }

    tf.translation = new_part_pos.extend(0.0);

    if mouse.just_pressed(MouseButton::Left) {
        log::debug!("Deselecting entity {}", entity);
        commands.queue(DeselectPart {
            id: entity,
            parent: new_part_parent,
        });
        sprite.color = Color::WHITE;
        *mouse_over = MouseOverState::Nothing;
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputState>();
        app.init_resource::<MouseOverState>();
        app.add_systems(OnEnter(AppState::Editing), build_editor_ui);
        app.add_systems(OnExit(AppState::Editing), remove_editor_ui);
        app.add_systems(
            Update,
            (
                handle_editor_input,
                system_hover
                    .run_if(|s: Res<MouseOverState>| !matches!(*s, MouseOverState::Selected(_))),
                system_drag
                    .run_if(|s: Res<MouseOverState>| matches!(*s, MouseOverState::Selected(_))),
            )
                .run_if(in_state(AppState::Editing)),
        );
    }
}
