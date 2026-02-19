use bevy::prelude::*;

use crate::components::item::ItemSpec;
use crate::components::ui::{
    InventoryRoot, InventorySlotNode, SelectedItemIndex, TransferChargeText,
};
use crate::config::GameConfig;
use crate::plugins::item::{inventory_capacity, rarity_prefix};
use crate::plugins::transfer::{
    add_future_transfer, add_past_transfer, can_transfer, minimum_target_floor,
};
use crate::resources::player_state::PlayerState;
use crate::resources::transfer_state::TransferState;
use crate::resources::{CurrentFloor, DungeonRng};
use crate::states::GameState;
use rand::Rng;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedItemIndex>()
            .add_systems(
                Update,
                open_inventory.run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::InventoryOpen), spawn_inventory_ui)
            .add_systems(OnExit(GameState::InventoryOpen), cleanup_inventory_ui)
            .add_systems(
                Update,
                inventory_navigation.run_if(in_state(GameState::InventoryOpen)),
            );
    }
}

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
) {
    selected.0 = 0;
    let capacity = inventory_capacity(&player_state.equipment, &config.item) as usize;

    commands
        .spawn((
            InventoryRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Inventory (I:close, Arrows:select, E:equip, F:future, P:past)"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent.spawn((
                TransferChargeText,
                Text::new(format!("Transfer Charges: {}", transfer_state.charges)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 1.0)),
            ));

            for i in 0..capacity {
                let item_text = format_slot_text(i, &player_state.inventory.slots[i], i == 0);
                parent.spawn((
                    InventorySlotNode(i),
                    Text::new(item_text),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            }
        });
}

fn format_slot_text(index: usize, spec: &Option<ItemSpec>, selected: bool) -> String {
    let cursor = if selected { "> " } else { "  " };
    match spec {
        Some(s) => format!(
            "{}[{}] {}{:?} Lv{} (+{})",
            cursor,
            index + 1,
            rarity_prefix(s.rarity),
            s.kind,
            s.level,
            s.value
        ),
        None => format!("{}[{}] ---", cursor, index + 1),
    }
}

#[allow(clippy::too_many_arguments)]
fn inventory_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut selected: ResMut<SelectedItemIndex>,
    mut player_state: ResMut<PlayerState>,
    mut transfer_state: ResMut<TransferState>,
    current_floor: Res<CurrentFloor>,
    config: Res<GameConfig>,
    mut rng: ResMut<DungeonRng>,
    mut slot_query: Query<(&InventorySlotNode, &mut Text)>,
    mut charge_query: Query<&mut Text, (With<TransferChargeText>, Without<InventorySlotNode>)>,
) {
    let capacity = inventory_capacity(&player_state.equipment, &config.item) as usize;

    if keyboard.just_pressed(KeyCode::Escape)
        || keyboard.just_pressed(KeyCode::KeyI)
        || keyboard.just_pressed(KeyCode::Tab)
    {
        next_state.set(GameState::Playing);
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) && selected.0 > 0 {
        selected.0 -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && selected.0 + 1 < capacity {
        selected.0 += 1;
    }

    // Equip (E)
    if keyboard.just_pressed(KeyCode::KeyE) {
        let idx = selected.0;
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
            info!("Equipped {:?} from inventory", spec.kind);
        }
    }

    // Future transfer (F)
    if keyboard.just_pressed(KeyCode::KeyF) {
        let idx = selected.0;
        if let Some(spec) = player_state.inventory.slots[idx]
            && can_transfer(&transfer_state)
        {
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

    // Past transfer (P)
    if keyboard.just_pressed(KeyCode::KeyP) {
        let idx = selected.0;
        if let Some(spec) = player_state.inventory.slots[idx]
            && can_transfer(&transfer_state)
        {
            let source = current_floor.number();
            if add_past_transfer(&mut transfer_state, spec, source) {
                player_state.inventory.slots[idx] = None;
                info!("Sent {:?} to past (next run)", spec.kind);
            }
        }
    }

    for (slot_node, mut text) in &mut slot_query {
        let i = slot_node.0;
        if i < capacity {
            **text = format_slot_text(i, &player_state.inventory.slots[i], i == selected.0);
        }
    }

    if let Ok(mut text) = charge_query.single_mut() {
        **text = format!("Transfer Charges: {}", transfer_state.charges);
    }
}

fn cleanup_inventory_ui(mut commands: Commands, query: Query<Entity, With<InventoryRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
