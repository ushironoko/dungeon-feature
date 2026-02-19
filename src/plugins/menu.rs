use bevy::prelude::*;

use crate::components::item::ItemSpec;
use crate::components::ui::{
    ContinueWithItemButton, EndingItemButton, EndingItemSelectRoot, EndingRoot, EndingSelectedItem,
    GameOverFloorText, GameOverRoot, MenuRoot, ReturnToMenuButton, StartButton, StartFreshButton,
};
use crate::components::FloorEntity;
use crate::config::GameConfig;
use crate::plugins::item::rarity_prefix;
use crate::plugins::transfer::add_ending_carryover;
use crate::resources::player_state::PlayerState;
use crate::resources::transfer_state::TransferState;
use crate::resources::{CurrentFloor, DungeonRng};
use crate::states::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EndingSelectedItem>()
            .add_systems(OnEnter(GameState::Menu), spawn_menu)
            .add_systems(OnExit(GameState::Menu), cleanup_menu)
            .add_systems(
                Update,
                menu_interaction.run_if(in_state(GameState::Menu)),
            )
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over)
            .add_systems(OnEnter(GameState::GameOver), cleanup_floor_on_game_over)
            .add_systems(OnExit(GameState::GameOver), cleanup_game_over)
            .add_systems(
                Update,
                game_over_interaction.run_if(in_state(GameState::GameOver)),
            )
            .add_systems(OnEnter(GameState::Ending), spawn_ending)
            .add_systems(OnEnter(GameState::Ending), cleanup_floor_on_ending)
            .add_systems(OnExit(GameState::Ending), cleanup_ending)
            .add_systems(
                Update,
                (ending_item_select, ending_button_interaction)
                    .run_if(in_state(GameState::Ending)),
            );
    }
}

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(32.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Dungeon Feature"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
            ));

            parent
                .spawn((
                    StartButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(40.0), Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Start"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn menu_interaction(
    query: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut current_floor: ResMut<CurrentFloor>,
    mut player_state: ResMut<PlayerState>,
    mut transfer_state: ResMut<TransferState>,
    mut rng: ResMut<DungeonRng>,
    config: Res<GameConfig>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            current_floor.reset();
            player_state.reset(config.player.hp);
            transfer_state.reset_for_new_run(config.transfer.charges_per_run, &mut rng.0);
            next_state.set(GameState::FloorTransition);
        }
    }
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn spawn_game_over(mut commands: Commands, current_floor: Res<CurrentFloor>) {
    commands
        .spawn((
            GameOverRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.9)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Game Over"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
            ));

            parent.spawn((
                GameOverFloorText,
                Text::new(format!("Reached Floor {}", current_floor.number())),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            parent
                .spawn((
                    ReturnToMenuButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(40.0), Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Return to Menu"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn game_over_interaction(
    query: Query<&Interaction, (Changed<Interaction>, With<ReturnToMenuButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Menu);
        }
    }
}

fn cleanup_game_over(mut commands: Commands, query: Query<Entity, With<GameOverRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn cleanup_floor_on_game_over(mut commands: Commands, query: Query<Entity, With<FloorEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn spawn_ending(
    mut commands: Commands,
    player_state: Res<PlayerState>,
    mut selected: ResMut<EndingSelectedItem>,
) {
    selected.0 = None;

    commands
        .spawn((
            EndingRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.15, 0.95)),
        ))
        .with_children(|parent| {
            // 物語テキスト
            parent.spawn((
                Text::new("The treasure chest is empty..."),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.7, 0.4)),
            ));

            parent.spawn((
                Text::new("But you can bring one item with you."),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));

            parent.spawn((
                Text::new("Select one item to keep:"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.8, 1.0)),
            ));

            // インベントリ + 装備からアイテムボタンを生成
            parent
                .spawn((
                    EndingItemSelectRoot,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(4.0),
                        max_height: Val::Px(300.0),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                ))
                .with_children(|select_root| {
                    let mut item_idx = 0usize;
                    // Equipment items
                    let eq_slots: [(&str, &Option<ItemSpec>); 7] = [
                        ("Weapon", &player_state.equipment.weapon),
                        ("Head", &player_state.equipment.head),
                        ("Torso", &player_state.equipment.torso),
                        ("Legs", &player_state.equipment.legs),
                        ("Shield", &player_state.equipment.shield),
                        ("Charm", &player_state.equipment.charm),
                        ("Backpack", &player_state.equipment.backpack),
                    ];
                    for (name, slot) in &eq_slots {
                        if let Some(spec) = slot {
                            spawn_ending_item_button(select_root, item_idx, name, spec);
                            item_idx += 1;
                        }
                    }
                    // Inventory items
                    for (i, slot) in player_state.inventory.slots.iter().enumerate() {
                        if let Some(spec) = slot {
                            let name_str = format!("Inv{}", i + 1);
                            spawn_ending_item_button(select_root, item_idx, &name_str, spec);
                            item_idx += 1;
                        }
                    }
                });

            // ボタン行
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    margin: UiRect::top(Val::Px(16.0)),
                    ..default()
                })
                .with_children(|row| {
                    // Continue with item ボタン
                    row.spawn((
                        ContinueWithItemButton,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(32.0), Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.4, 0.3)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("Continue with item"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        ));
                    });

                    // Start fresh ボタン
                    row.spawn((
                        StartFreshButton,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(32.0), Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.4, 0.3, 0.3)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("Start fresh"),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
        });
}

fn spawn_ending_item_button(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    name: &str,
    spec: &ItemSpec,
) {
    parent
        .spawn((
            EndingItemButton(index),
            Button,
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(format!(
                    "{}: {}{:?} Lv{} (+{})",
                    name,
                    rarity_prefix(spec.rarity),
                    spec.kind,
                    spec.level,
                    spec.value
                )),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

fn ending_item_select(
    query: Query<(&Interaction, &EndingItemButton), Changed<Interaction>>,
    mut selected: ResMut<EndingSelectedItem>,
    player_state: Res<PlayerState>,
    mut btn_query: Query<(&EndingItemButton, &mut BackgroundColor)>,
) {
    for (interaction, btn) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // ボタンインデックスから実際のアイテムを特定
        let mut item_idx = 0usize;
        let mut found_spec: Option<ItemSpec> = None;

        // Equipment items
        let eq_items: [&Option<ItemSpec>; 7] = [
            &player_state.equipment.weapon,
            &player_state.equipment.head,
            &player_state.equipment.torso,
            &player_state.equipment.legs,
            &player_state.equipment.shield,
            &player_state.equipment.charm,
            &player_state.equipment.backpack,
        ];
        for spec in eq_items.iter().copied().flatten() {
            if item_idx == btn.0 {
                found_spec = Some(*spec);
                break;
            }
            item_idx += 1;
        }

        // Inventory items
        if found_spec.is_none() {
            for spec in player_state.inventory.slots.iter().flatten() {
                if item_idx == btn.0 {
                    found_spec = Some(*spec);
                    break;
                }
                item_idx += 1;
            }
        }

        if let Some(spec) = found_spec {
            selected.0 = Some(spec);
            info!("Selected {:?} Lv{} for next run", spec.kind, spec.level);

            // ハイライト更新
            for (item_btn, mut bg) in &mut btn_query {
                if item_btn.0 == btn.0 {
                    *bg = BackgroundColor(Color::srgb(0.3, 0.5, 0.6));
                } else {
                    *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.3));
                }
            }
        }
    }
}

fn ending_button_interaction(
    continue_query: Query<&Interaction, (Changed<Interaction>, With<ContinueWithItemButton>)>,
    fresh_query: Query<&Interaction, (Changed<Interaction>, With<StartFreshButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut transfer_state: ResMut<TransferState>,
    selected: Res<EndingSelectedItem>,
) {
    // Continue with item
    for interaction in &continue_query {
        if *interaction == Interaction::Pressed {
            if let Some(spec) = selected.0 {
                add_ending_carryover(&mut transfer_state, spec);
                info!("Bringing {:?} Lv{} to next run!", spec.kind, spec.level);
            }
            next_state.set(GameState::Menu);
        }
    }

    // Start fresh
    for interaction in &fresh_query {
        if *interaction == Interaction::Pressed {
            // past_items をクリア（アイテムなしで次周回）
            transfer_state.past_items = [None; 32];
            next_state.set(GameState::Menu);
        }
    }
}

fn cleanup_ending(mut commands: Commands, query: Query<Entity, With<EndingRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn cleanup_floor_on_ending(mut commands: Commands, query: Query<Entity, With<FloorEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
