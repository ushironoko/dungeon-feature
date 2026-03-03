use bevy::prelude::*;

use crate::components::item::{EquipSlot, ItemKind, ItemSpec};
use crate::components::ui::{
    ContextMenu, ContextMenuData, ContextMenuItem, ContextMenuState, EquipmentPanel,
    EquipmentSlotNode, InventoryPanel, InventoryRoot, InventorySlotNode, ItemAction,
    SelectedItemIndex, TransferChargeText,
};
use crate::config::GameConfig;
use crate::plugins::item::{
    equip_slot_label, inventory_capacity, item_kind_label, rarity_color, rarity_prefix,
};
use crate::plugins::transfer::{add_future_transfer, add_past_transfer, minimum_target_floor};
use crate::resources::font::GameFont;
use crate::resources::player_state::PlayerState;
use crate::resources::transfer_state::TransferState;
use crate::resources::{CurrentFloor, DungeonRng};
use crate::states::GameState;
use rand::Rng;

pub struct InventoryPlugin;

fn menu_closed(state: Res<ContextMenuState>) -> bool {
    state.0.is_none()
}
fn menu_open(state: Res<ContextMenuState>) -> bool {
    state.0.is_some()
}

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedItemIndex>()
            .init_resource::<ContextMenuState>()
            .add_systems(Update, open_inventory.run_if(in_state(GameState::Playing)))
            .add_systems(OnEnter(GameState::InventoryOpen), spawn_inventory_ui)
            .add_systems(OnExit(GameState::InventoryOpen), cleanup_inventory_ui)
            .add_systems(
                Update,
                (
                    // Browse モード（メニュー閉）
                    (
                        inventory_slot_navigation,
                        open_context_menu,
                        close_inventory,
                    )
                        .run_if(menu_closed),
                    // Menu モード（メニュー開）
                    (
                        context_menu_navigation,
                        execute_context_action,
                        close_context_menu,
                    )
                        .run_if(menu_open),
                    // 常時（表示更新）
                    update_slot_display,
                    update_equipment_display,
                )
                    .run_if(in_state(GameState::InventoryOpen)),
            );
    }
}

// --- 純粋関数 ---

const EQUIP_SLOTS: [EquipSlot; 7] = [
    EquipSlot::Weapon,
    EquipSlot::Head,
    EquipSlot::Torso,
    EquipSlot::Legs,
    EquipSlot::Shield,
    EquipSlot::Charm,
    EquipSlot::Backpack,
];

fn build_actions(spec: &ItemSpec) -> ([ItemAction; 6], usize) {
    let mut actions = [ItemAction::Close; 6];
    let mut count = 0;

    match spec.kind {
        ItemKind::HealthPotion => {
            actions[count] = ItemAction::Use;
            count += 1;
        }
        _ if spec.kind.equip_slot().is_some() => {
            actions[count] = ItemAction::Equip;
            count += 1;
        }
        _ => {}
    }
    actions[count] = ItemAction::SendFuture;
    count += 1;
    actions[count] = ItemAction::SendPast;
    count += 1;
    actions[count] = ItemAction::Discard;
    count += 1;
    actions[count] = ItemAction::Close;
    count += 1;

    (actions, count)
}

fn action_label(action: ItemAction, charges: u32) -> String {
    match action {
        ItemAction::Equip => "装備する".to_string(),
        ItemAction::Use => "使う".to_string(),
        ItemAction::SendFuture if charges > 0 => format!("未来に送る (残:{})", charges),
        ItemAction::SendFuture => "未来に送る (不可)".to_string(),
        ItemAction::SendPast if charges > 0 => format!("過去に送る (残:{})", charges),
        ItemAction::SendPast => "過去に送る (不可)".to_string(),
        ItemAction::Discard => "捨てる".to_string(),
        ItemAction::Close => "閉じる".to_string(),
    }
}

fn format_slot_text(index: usize, spec: &Option<ItemSpec>, selected: bool) -> String {
    let cursor = if selected { "> " } else { "  " };
    match spec {
        Some(s) => format!(
            "{}[{}] {}{} Lv{} (+{})",
            cursor,
            index + 1,
            rarity_prefix(s.rarity),
            item_kind_label(s.kind),
            s.level,
            s.value
        ),
        None => format!("{}[{}] ---", cursor, index + 1),
    }
}

fn format_equip_slot_text(slot: EquipSlot, spec: Option<&ItemSpec>) -> String {
    let label = equip_slot_label(slot);
    match spec {
        Some(s) => format!(
            "{}: {}{} Lv{} (+{})",
            label,
            rarity_prefix(s.rarity),
            item_kind_label(s.kind),
            s.level,
            s.value
        ),
        None => format!("{}: ---", label),
    }
}

// --- Systems ---

fn open_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::KeyI) || keyboard.just_pressed(KeyCode::Tab) {
        next_state.set(GameState::InventoryOpen);
    }
}

fn spawn_inventory_ui(
    mut commands: Commands,
    player_state: Res<PlayerState>,
    transfer_state: Res<TransferState>,
    config: Res<GameConfig>,
    mut selected: ResMut<SelectedItemIndex>,
    mut ctx_state: ResMut<ContextMenuState>,
    game_font: Res<GameFont>,
) {
    selected.0 = 0;
    ctx_state.0 = None;
    let capacity = inventory_capacity(&player_state.equipment, &config.item) as usize;
    let font = game_font.0.clone();

    commands
        .spawn((
            InventoryRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|root| {
            // ヘッダー行
            root.spawn(Node {
                width: Val::Px(720.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("インベントリ (I:閉じる)"),
                    TextFont {
                        font: font.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                header.spawn((
                    TransferChargeText,
                    Text::new(format!("転送回数: {}", transfer_state.charges)),
                    TextFont {
                        font: font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.5, 0.8, 1.0)),
                ));
            });

            // メインコンテンツ（2パネル）
            root.spawn(Node {
                width: Val::Px(720.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|panels| {
                // 左パネル: 装備中
                panels
                    .spawn((
                        EquipmentPanel,
                        Node {
                            width: Val::Px(280.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
                    ))
                    .with_children(|equip_panel| {
                        equip_panel.spawn((
                            Text::new("【装備中】"),
                            TextFont {
                                font: font.clone(),
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.5)),
                        ));

                        for slot in &EQUIP_SLOTS {
                            let spec = player_state.equipment.get(*slot);
                            let text = format_equip_slot_text(*slot, spec);
                            let color = spec
                                .map(|s| rarity_color(s.rarity))
                                .unwrap_or(Color::srgb(0.5, 0.5, 0.5));
                            equip_panel.spawn((
                                EquipmentSlotNode(*slot),
                                Text::new(text),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(color),
                            ));
                        }
                    });

                // 右パネル: 所持品（相対配置でコンテキストメニューを重ねる）
                panels
                    .spawn(Node {
                        width: Val::Px(420.0),
                        flex_direction: FlexDirection::Column,
                        position_type: PositionType::Relative,
                        ..default()
                    })
                    .with_children(|right_wrapper| {
                        right_wrapper
                            .spawn((
                                InventoryPanel,
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(8.0)),
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
                            ))
                            .with_children(|inv_panel| {
                                inv_panel.spawn((
                                    Text::new("【所持品】"),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.9, 0.9, 0.5)),
                                ));

                                for i in 0..capacity {
                                    let item_text = format_slot_text(
                                        i,
                                        &player_state.inventory.slots[i],
                                        i == 0,
                                    );
                                    let color = player_state.inventory.slots[i]
                                        .as_ref()
                                        .map(|s| rarity_color(s.rarity))
                                        .unwrap_or(Color::srgb(0.5, 0.5, 0.5));
                                    inv_panel.spawn((
                                        InventorySlotNode(i),
                                        Node {
                                            padding: UiRect::horizontal(Val::Px(4.0)),
                                            ..default()
                                        },
                                        Text::new(item_text),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 13.0,
                                            ..default()
                                        },
                                        TextColor(color),
                                        BackgroundColor(if i == 0 {
                                            Color::srgba(0.3, 0.3, 0.5, 0.8)
                                        } else {
                                            Color::NONE
                                        }),
                                    ));
                                }
                            });
                    });
            });
        });
}

// Browse モード: スロットナビゲーション
fn inventory_slot_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedItemIndex>,
    player_state: Res<PlayerState>,
    config: Res<GameConfig>,
) {
    let capacity = inventory_capacity(&player_state.equipment, &config.item) as usize;

    if keyboard.just_pressed(KeyCode::ArrowUp) && selected.0 > 0 {
        selected.0 -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && selected.0 + 1 < capacity {
        selected.0 += 1;
    }
}

// Browse モード: コンテキストメニューを開く
#[allow(clippy::too_many_arguments)]
fn open_context_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedItemIndex>,
    player_state: Res<PlayerState>,
    transfer_state: Res<TransferState>,
    mut ctx_state: ResMut<ContextMenuState>,
    mut commands: Commands,
    inv_panel_query: Query<Entity, With<InventoryPanel>>,
    game_font: Res<GameFont>,
) {
    if !keyboard.just_pressed(KeyCode::Enter) && !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    let idx = selected.0;
    let Some(spec) = player_state.inventory.slots[idx] else {
        return; // 空スロットではメニューを開かない
    };

    let (actions, action_count) = build_actions(&spec);
    ctx_state.0 = Some(ContextMenuData {
        selected: 0,
        actions,
        action_count,
    });

    // コンテキストメニューUIをスポーン
    let Ok(panel_entity) = inv_panel_query.single() else {
        return;
    };

    commands.entity(panel_entity).with_children(|parent| {
        parent
            .spawn((
                ContextMenu,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(8.0),
                    bottom: Val::Px(8.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    row_gap: Val::Px(2.0),
                    min_width: Val::Px(180.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.95)),
                ZIndex(10),
            ))
            .with_children(|menu| {
                for (i, &action) in actions.iter().enumerate().take(action_count) {
                    let label = action_label(action, transfer_state.charges);
                    let is_disabled =
                        matches!(action, ItemAction::SendFuture | ItemAction::SendPast)
                            && transfer_state.charges == 0;
                    let text_color = if is_disabled {
                        Color::srgb(0.4, 0.4, 0.4)
                    } else {
                        Color::WHITE
                    };
                    let cursor = if i == 0 { "> " } else { "  " };

                    menu.spawn((
                        ContextMenuItem(action),
                        Node {
                            padding: UiRect::horizontal(Val::Px(4.0)),
                            ..default()
                        },
                        Text::new(format!("{}{}", cursor, label)),
                        TextFont {
                            font: game_font.0.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(text_color),
                        BackgroundColor(if i == 0 {
                            Color::srgba(0.3, 0.3, 0.5, 0.8)
                        } else {
                            Color::NONE
                        }),
                    ));
                }
            });
    });
}

// Browse モード: インベントリを閉じる
fn close_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape)
        || keyboard.just_pressed(KeyCode::KeyI)
        || keyboard.just_pressed(KeyCode::Tab)
    {
        next_state.set(GameState::Playing);
    }
}

// Menu モード: メニュー項目ナビゲーション
fn context_menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ctx_state: ResMut<ContextMenuState>,
    mut menu_query: Query<(&ContextMenuItem, &mut Text, &mut BackgroundColor)>,
) {
    let Some(ref mut data) = ctx_state.0 else {
        return;
    };

    let old_selected = data.selected;
    if keyboard.just_pressed(KeyCode::ArrowUp) && data.selected > 0 {
        data.selected -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && data.selected + 1 < data.action_count {
        data.selected += 1;
    }

    if old_selected != data.selected {
        let actions_snapshot: Vec<_> = data.actions[..data.action_count].to_vec();
        let new_selected = data.selected;
        for (item, mut text, mut bg) in &mut menu_query {
            if let Some(idx) = actions_snapshot.iter().position(|a| *a == item.0) {
                let is_sel = idx == new_selected;
                // テキストのカーソルプレフィクスだけ更新
                let current = text.0.clone();
                let trimmed = current.trim_start_matches("> ").trim_start();
                **text = if is_sel {
                    format!("> {}", trimmed)
                } else {
                    format!("  {}", trimmed)
                };
                *bg = if is_sel {
                    BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.8))
                } else {
                    BackgroundColor(Color::NONE)
                };
            }
        }
    }
}

// Menu モード: アクション実行
#[allow(clippy::too_many_arguments)]
fn execute_context_action(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ctx_state: ResMut<ContextMenuState>,
    mut selected: ResMut<SelectedItemIndex>,
    mut player_state: ResMut<PlayerState>,
    mut transfer_state: ResMut<TransferState>,
    current_floor: Res<CurrentFloor>,
    config: Res<GameConfig>,
    mut rng: ResMut<DungeonRng>,
    menu_query: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
    mut health_query: Query<&mut crate::components::Health, With<crate::components::Player>>,
) {
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }

    let Some(ref data) = ctx_state.0 else {
        return;
    };

    let action = data.actions[data.selected];
    let idx = selected.0;

    match action {
        ItemAction::Equip => {
            if let Some(spec) = player_state.inventory.slots[idx]
                && let Some(slot) = spec.kind.equip_slot()
            {
                let old = player_state.equipment.set(slot, Some(spec));
                player_state.inventory.slots[idx] = None;
                if let Some(old_spec) = old {
                    player_state.inventory.slots[idx] = Some(old_spec);
                }
                player_state.inventory.capacity =
                    inventory_capacity(&player_state.equipment, &config.item);
                // 容量変更後のクランプ
                let new_cap = player_state.inventory.capacity as usize;
                if selected.0 >= new_cap {
                    selected.0 = new_cap.saturating_sub(1);
                }
                info!("Equipped {:?} from inventory", spec.kind);
            }
        }
        ItemAction::Use => {
            if let Some(spec) = player_state.inventory.slots[idx]
                && spec.kind == ItemKind::HealthPotion
                && let Ok(mut health) = health_query.single_mut()
            {
                let old_hp = health.current;
                health.current = (health.current + spec.value).min(health.max);
                player_state.inventory.slots[idx] = None;
                info!("Used HealthPotion! HP: {} -> {}", old_hp, health.current);
            }
        }
        ItemAction::SendFuture => {
            if transfer_state.charges == 0 {
                // 実行不可
            } else if let Some(spec) = player_state.inventory.slots[idx] {
                let source = current_floor.number();
                let min_target = minimum_target_floor(&spec).max(source + 1);
                let max_floor = config.dungeon.max_floor;
                if min_target <= max_floor {
                    let target = rng.0.random_range(min_target..=max_floor);
                    if add_future_transfer(&mut transfer_state, spec, source, target) {
                        player_state.inventory.slots[idx] = None;
                        info!("Sent {:?} to future (floor {})", spec.kind, target);
                    }
                }
            }
        }
        ItemAction::SendPast => {
            if transfer_state.charges == 0 {
                // 実行不可
            } else if let Some(spec) = player_state.inventory.slots[idx] {
                let source = current_floor.number();
                if add_past_transfer(&mut transfer_state, spec, source) {
                    player_state.inventory.slots[idx] = None;
                    info!("Sent {:?} to past (next run)", spec.kind);
                }
            }
        }
        ItemAction::Discard => {
            if player_state.inventory.slots[idx].is_some() {
                let spec = player_state.inventory.slots[idx].unwrap();
                player_state.inventory.slots[idx] = None;
                info!("Discarded {:?} Lv{}", spec.kind, spec.level);
            }
        }
        ItemAction::Close => {}
    }

    // メニューを閉じる
    ctx_state.0 = None;
    for entity in &menu_query {
        commands.entity(entity).despawn();
    }
}

// Menu モード: Escape でメニューを閉じる
fn close_context_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ctx_state: ResMut<ContextMenuState>,
    menu_query: Query<Entity, With<ContextMenu>>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        ctx_state.0 = None;
        for entity in &menu_query {
            commands.entity(entity).despawn();
        }
    }
}

// 常時: スロット表示更新
#[allow(clippy::type_complexity)]
fn update_slot_display(
    selected: Res<SelectedItemIndex>,
    player_state: Res<PlayerState>,
    config: Res<GameConfig>,
    mut slot_query: Query<(
        &InventorySlotNode,
        &mut Text,
        &mut TextColor,
        &mut BackgroundColor,
    )>,
    mut charge_query: Query<
        &mut Text,
        (
            With<TransferChargeText>,
            Without<InventorySlotNode>,
            Without<EquipmentSlotNode>,
            Without<ContextMenuItem>,
        ),
    >,
    transfer_state: Res<TransferState>,
) {
    if !selected.is_changed() && !player_state.is_changed() && !transfer_state.is_changed() {
        return;
    }
    let capacity = inventory_capacity(&player_state.equipment, &config.item) as usize;

    for (slot_node, mut text, mut text_color, mut bg) in &mut slot_query {
        let i = slot_node.0;
        if i < capacity {
            let is_selected = i == selected.0;
            **text = format_slot_text(i, &player_state.inventory.slots[i], is_selected);
            let color = player_state.inventory.slots[i]
                .as_ref()
                .map(|s| rarity_color(s.rarity))
                .unwrap_or(Color::srgb(0.5, 0.5, 0.5));
            *text_color = TextColor(color);
            *bg = if is_selected {
                BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.8))
            } else {
                BackgroundColor(Color::NONE)
            };
        }
    }

    if let Ok(mut text) = charge_query.single_mut() {
        **text = format!("転送回数: {}", transfer_state.charges);
    }
}

// 常時: 装備パネル表示更新
fn update_equipment_display(
    player_state: Res<PlayerState>,
    mut equip_query: Query<(&EquipmentSlotNode, &mut Text, &mut TextColor)>,
) {
    if !player_state.is_changed() {
        return;
    }
    for (slot_node, mut text, mut text_color) in &mut equip_query {
        let spec = player_state.equipment.get(slot_node.0);
        **text = format_equip_slot_text(slot_node.0, spec);
        let color = spec
            .map(|s| rarity_color(s.rarity))
            .unwrap_or(Color::srgb(0.5, 0.5, 0.5));
        *text_color = TextColor(color);
    }
}

fn cleanup_inventory_ui(
    mut commands: Commands,
    query: Query<Entity, With<InventoryRoot>>,
    mut ctx_state: ResMut<ContextMenuState>,
) {
    ctx_state.0 = None;
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
