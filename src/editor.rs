
use bevy::prelude::*;
use crate::*;

// ── Editor components ─────────────────────────────────────────────────────────

#[derive(Component)]
pub struct EditorRoot;

#[derive(Component)]
pub struct StageButton(Entity);

#[derive(Component)]
pub struct AddStageButton;


// ── Rocket editor ─────────────────────────────────────────────────────────────

// Spawn / despawn the editor panel. Rebuilds only when the stage layout changes.
// pub fn rebuild_editor_ui(
//     mut commands: Commands,
//     rocket_q: Query<&Rocket, With<PlayerRocket>>,
//     body_q: Query<&CelestialBody>,
//     // stage_q: Query<&Children, With<RocketStage>>,
//     tank_check: Query<(), With<FuelTank>>,
//     existing_editor: Query<Entity, With<EditorRoot>>,
//     mut prev_state: Local<Option<(bool, Vec<(Entity, usize)>)>>,
// ) {
//     let show_editor = rocket_q
//         .get_single()
//         .ok()
//         .map(|rocket| {
//             rocket.landed
//                 && rocket
//                     .soi_body
//                     .and_then(|e| body_q.get(e).ok())
//                     .map(|b| b.parent.is_none())
//                     .unwrap_or(false)
//         })
//         .unwrap_or(false);

//     // Build snapshot of current stage configuration
//     let current_stages: Vec<(Entity, usize)> = if show_editor {
//         let rocket = rocket_q.get_single().unwrap();
//         rocket
//             .stage_queue
//             .iter()
//             .copied()
//             .rev()
//             .chain(rocket.active_stage)
//             .map(|e| {
//                 let tanks = stage_q
//                     .get(e)
//                     .map(|ch| ch.iter().filter(|&c| tank_check.get(*c).is_ok()).count())
//                     .unwrap_or(0);
//                 (e, tanks)
//             })
//             .collect()
//     } else {
//         vec![]
//     };

//     let current = (show_editor, current_stages);
//     if prev_state.as_ref() == Some(&current) {
//         return;
//     }
//     *prev_state = Some(current.clone());

//     // Tear down old editor
//     for e in &existing_editor {
//         commands.entity(e).despawn_recursive();
//     }

//     if !show_editor {
//         return;
//     }

//     let (_, ref stages) = current;

//     commands
//         .spawn((
//             EditorRoot,
//             Node {
//                 position_type: PositionType::Absolute,
//                 right: Val::Px(12.0),
//                 top: Val::Px(80.0),
//                 flex_direction: FlexDirection::Column,
//                 row_gap: Val::Px(4.0),
//                 padding: UiRect::all(Val::Px(8.0)),
//                 ..default()
//             },
//             BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.85)),
//         ))
//         .with_children(|root| {
//             // Title
//             root.spawn((
//                 Text::new("ROCKET EDITOR"),
//                 TextFont {
//                     font_size: 14.0,
//                     ..default()
//                 },
//                 TextColor(Color::srgb(0.8, 0.8, 0.8)),
//             ));

//             // One button per stage (top-to-bottom)
//             for (i, &(stage_entity, tank_count)) in stages.iter().enumerate() {
//                 root.spawn((
//                     Text::new(format!("Stage {} [{} tanks]", i + 1, tank_count)),
//                     TextFont {
//                         font_size: 16.0,
//                         ..default()
//                     },
//                     TextColor(Color::srgb(0.9, 0.9, 0.5)),
//                     Node {
//                         padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
//                         min_width: Val::Px(170.0),
//                         ..default()
//                     },
//                     BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
//                     Interaction::default(),
//                     StageButton(stage_entity),
//                 ));
//             }

//             // "Add stage" button
//             root.spawn((
//                 Text::new("+ Add Stage"),
//                 TextFont {
//                     font_size: 16.0,
//                     ..default()
//                 },
//                 TextColor(Color::srgb(0.5, 1.0, 0.5)),
//                 Node {
//                     padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
//                     min_width: Val::Px(170.0),
//                     ..default()
//                 },
//                 BackgroundColor(Color::srgba(0.15, 0.25, 0.15, 0.9)),
//                 Interaction::default(),
//                 AddStageButton,
//             ));
//         });
// }

// /// Process left-click (add tank) and right-click (remove tank / stage) on
// /// editor buttons, plus the "add stage" button.
// pub fn handle_editor_input(
//     mut commands: Commands,
//     rocket_assets: Res<RocketAssets>,
//     mouse: Res<ButtonInput<MouseButton>>,
//     mut rocket_q: Query<(Entity, &mut Rocket), With<PlayerRocket>>,
//     stage_q: Query<&Parent, With<RocketStage>>,
//     tank_check: Query<(), With<FuelTank>>,
//     stage_buttons: Query<(&Interaction, &StageButton)>,
//     add_button: Query<&Interaction, With<AddStageButton>>,
// ) {
//     let Ok((_rocket_entity, mut rocket)) = rocket_q.get_single_mut() else {
//         return;
//     };

//     // Stage buttons — left click = add tank, right click = remove tank/stage
//     for (interaction, stage_btn) in &stage_buttons {
//         let stage_entity = stage_btn.0;

//         // Left click: add a fuel tank
//         if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
//             let tank = commands
//                 .spawn((
//                     rocket_assets.tank_sprite.clone(),
//                     Transform::default(),
//                     FuelTank {
//                         fuel: DEFAULT_FUEL_PER_TANK,
//                         capacity: DEFAULT_FUEL_PER_TANK,
//                     },
//                 ))
//                 .id();
//             commands.entity(stage_entity).add_child(tank);
//         }

//         // Right click: remove a fuel tank, or remove the whole stage if empty
//         if *interaction == Interaction::Hovered && mouse.just_pressed(MouseButton::Right) {
//             let Ok(children) = stage_q.get(stage_entity) else {
//                 continue;
//             };
//             let mut tank_to_remove = None;
//             for &c in children.iter() {
//                 if tank_check.get(c).is_ok() {
//                     tank_to_remove = Some(c);
//                     break;
//                 }
//             }

//             if let Some(tank) = tank_to_remove {
//                 commands.entity(tank).despawn_recursive();
//             } else {
//                 // No tanks left — remove the entire stage
//                 if rocket.active_stage == Some(stage_entity) {
//                     rocket.active_stage = if !rocket.stage_queue.is_empty() {
//                         Some(rocket.stage_queue.remove(0))
//                     } else {
//                         None
//                     };
//                 } else {
//                     rocket.stage_queue.retain(|&e| e != stage_entity);
//                 }
//                 commands.entity(stage_entity).despawn_recursive();
//             }
//         }
//     }

//     // "Add stage" button
//     for interaction in &add_button {
//         if *interaction == Interaction::Pressed && mouse.just_pressed(MouseButton::Left) {
//             let (new_stage, new_tail) = build_stage(
//                 &mut commands,
//                 &rocket_assets,
//                 // 1,
//                 DEFAULT_FUEL_PER_TANK,
//                 DEFAULT_THRUST,
//                 None,
//                 Vec2::new(0.0, ENGINE_SIZE.y / 2.0),
//             );
//             commands.entity(rocket.tail.unwrap()).add_child(new_stage);
//             rocket.tail = Some(new_tail);

//             // New stage goes to the bottom (fires first = active_stage)
//             if let Some(old_active) = rocket.active_stage.take() {
//                 rocket.stage_queue.insert(0, old_active);
//             }
//             rocket.active_stage = Some(new_stage);
//         }
//     }
// }
