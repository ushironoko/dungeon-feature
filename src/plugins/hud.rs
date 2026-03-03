use bevy::prelude::*;

use crate::components::hud::{
    DashStateText, EquipmentText, FloorText, HpBarFill, HudRoot, HudTransferChargeText,
};
use crate::components::{DashActive, Health, Player};
use crate::plugins::item::rarity_prefix;
use crate::resources::CurrentFloor;
use crate::resources::font::GameFont;
use crate::resources::player_state::PlayerState;
use crate::resources::transfer_state::TransferState;
use crate::states::{GameState, PlayingSet};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_hud)
            .add_systems(OnEnter(GameState::GameOver), cleanup_hud)
            .add_systems(OnEnter(GameState::Ending), cleanup_hud)
            .add_systems(
                Update,
                (
                    update_hp_bar,
                    update_floor_text,
                    update_equipment_text,
                    update_transfer_charge_text,
                    update_dash_state_text,
                )
                    .in_set(PlayingSet::PostCombat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_hud(
    mut commands: Commands,
    existing: Query<Entity, With<HudRoot>>,
    game_font: Res<GameFont>,
) {
    if !existing.is_empty() {
        return;
    }

    let font = game_font.0.clone();
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // HP バー背景
            parent
                .spawn(Node {
                    width: Val::Px(200.0),
                    height: Val::Px(16.0),
                    ..default()
                })
                .insert(BackgroundColor(Color::srgb(0.2, 0.2, 0.2)))
                .with_children(|bar_bg| {
                    bar_bg
                        .spawn((
                            HpBarFill,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                        ))
                        .insert(BackgroundColor(Color::srgb(0.8, 0.1, 0.1)));
                });

            // フロアテキスト
            parent.spawn((
                FloorText,
                Text::new("Floor: 1"),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // 装備テキスト
            parent.spawn((
                EquipmentText,
                Text::new("Equipment: None"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // 転送回数テキスト
            parent.spawn((
                HudTransferChargeText,
                Text::new("Transfer: 5"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 1.0)),
            ));

            // ダッシュ状態テキスト
            parent.spawn((
                DashStateText,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.3)),
            ));
        });
}

fn cleanup_hud(mut commands: Commands, query: Query<Entity, With<HudRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn update_hp_bar(
    player_query: Query<&Health, (With<Player>, Changed<Health>)>,
    mut bar_query: Query<&mut Node, With<HpBarFill>>,
) {
    let Ok(health) = player_query.single() else {
        return;
    };
    let Ok(mut node) = bar_query.single_mut() else {
        return;
    };

    let ratio = if health.max > 0 {
        health.current as f32 / health.max as f32
    } else {
        0.0
    };
    node.width = Val::Percent(ratio * 100.0);
}

fn update_floor_text(
    current_floor: Res<CurrentFloor>,
    mut query: Query<&mut Text, With<FloorText>>,
) {
    if !current_floor.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    **text = format!("Floor: {}", current_floor.number());
}

fn format_slot(name: &str, spec: Option<&crate::components::item::ItemSpec>) -> String {
    match spec {
        Some(s) => format!("{}:{}(+{})", name, rarity_prefix(s.rarity), s.value),
        None => format!("{}:-", name),
    }
}

fn update_equipment_text(
    player_state: Res<PlayerState>,
    mut query: Query<&mut Text, With<EquipmentText>>,
) {
    if !player_state.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let eq = &player_state.equipment;
    let parts = [
        format_slot("W", eq.weapon.as_ref()),
        format_slot("H", eq.head.as_ref()),
        format_slot("T", eq.torso.as_ref()),
        format_slot("L", eq.legs.as_ref()),
        format_slot("S", eq.shield.as_ref()),
        format_slot("C", eq.charm.as_ref()),
        format_slot("B", eq.backpack.as_ref()),
    ];
    **text = parts.join(" | ");
}

fn update_transfer_charge_text(
    transfer_state: Res<TransferState>,
    mut query: Query<&mut Text, With<HudTransferChargeText>>,
) {
    if !transfer_state.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    **text = format!("Transfer: {}", transfer_state.charges);
}

fn update_dash_state_text(dash: Res<DashActive>, mut query: Query<&mut Text, With<DashStateText>>) {
    if !dash.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    **text = if dash.0 {
        "DASH".to_string()
    } else {
        String::new()
    };
}
