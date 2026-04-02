#![allow(unused)]

use crate::*;
use bevy::prelude::*;

// ── Editor components ─────────────────────────────────────────────────────────

#[derive(Component)]
pub struct EditorRoot;

#[derive(Component)]
pub struct PartButton(PartDef);

// ── Rocket editor ─────────────────────────────────────────────────────────────

// Spawn / despawn the editor panel. Rebuilds only when the stage layout changes.
pub fn build_editor_ui(
    mut commands: Commands,
    parts: Res<PartsCatalog>,
    // rocket_q: Query<&Rocket, With<PlayerRocket>>,
    // body_q: Query<&CelestialBody>,
    // stage_q: Query<&Children, With<RocketStage>>,
    // tank_check: Query<(), With<FuelTank>>,
    // existing_editor: Query<Entity, With<EditorRoot>>,
    // mut prev_state: Local<Option<(bool, Vec<(Entity, usize)>)>>,
) {
    // let show_editor = rocket_q
    //     .get_single()
    //     .ok()
    //     .map(|rocket| {
    //         rocket.landed
    //             && rocket
    //                 .soi_body
    //                 .and_then(|e| body_q.get(e).ok())
    //                 .map(|b| b.parent.is_none())
    //                 .unwrap_or(false)
    //     })
    //     .unwrap_or(false);

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
        ))
        .with_children(|root| {
            // Title
            root.spawn((
                Text::new("ROCKET EDITOR"),
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

pub fn handle_editor_input(
    mut commands: Commands,
    mut rocket_q: Query<(Entity, &mut Rocket), With<PlayerRocket>>,
    stage_buttons: Query<(&Interaction, &PartButton)>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Ok((rocket_entity, mut rocket)) = rocket_q.get_single_mut() else {
        return;
    };

    // Stage buttons — left click = add tank, right click = remove tank/stage
    for (interaction, part_btn) in &stage_buttons {
        // Left click: add a fuel tank
        if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
            commands.queue(RocketBuildCommand {
                rocket: rocket_entity,
                part_id: part_btn.0.id.clone(),
            });
        }
    }
}


// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing), build_editor_ui);
        app.add_systems(
            Update,
            (handle_editor_input,).run_if(in_state(AppState::Playing)),
        );
    }
}
