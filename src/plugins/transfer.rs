use bevy::prelude::*;

use crate::components::item::{DroppedItem, Item, ItemLevel, ItemRarity, ItemSpec, ItemType};
use crate::config::GameConfig;
use crate::plugins::item::{rarity_color, recompute_item_value};
use crate::resources::sprite_assets::{SpriteAssets, make_sprite};
use crate::resources::transfer_state::{FutureTransferItem, PastTransferItem, TransferState};
use crate::resources::{CurrentFloor, DungeonRng, FloorMap};
use crate::states::FloorTransitionSetup;
use crate::states::GameState;
use rand::Rng;

pub struct TransferPlugin;

impl Plugin for TransferPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::FloorTransition),
            spawn_transferred_items.in_set(FloorTransitionSetup::SpawnEntities),
        );
    }
}

// --- 純粋関数 ---

pub fn future_transfer_level(source_floor: u32, target_floor: u32) -> u32 {
    target_floor + (target_floor - source_floor)
}

pub fn minimum_target_floor(spec: &ItemSpec) -> u32 {
    spec.level + 1
}

pub fn can_transfer(state: &TransferState) -> bool {
    state.charges > 0
}

pub fn add_future_transfer(
    state: &mut TransferState,
    spec: ItemSpec,
    source: u32,
    target: u32,
) -> bool {
    for slot in &mut state.future_items {
        if slot.is_none() {
            *slot = Some(FutureTransferItem {
                spec,
                source_floor: source,
                target_floor: target,
            });
            state.charges -= 1;
            return true;
        }
    }
    false
}

pub fn add_past_transfer(state: &mut TransferState, spec: ItemSpec, source_floor: u32) -> bool {
    if state.charges == 0 {
        return false;
    }
    for slot in &mut state.past_items {
        if slot.is_none() {
            *slot = Some(PastTransferItem { spec, source_floor });
            state.charges -= 1;
            return true;
        }
    }
    false
}

pub fn add_ending_carryover(state: &mut TransferState, spec: ItemSpec) -> bool {
    for slot in &mut state.past_items {
        if slot.is_none() {
            *slot = Some(PastTransferItem {
                spec,
                source_floor: 1,
            });
            return true;
        }
    }
    false
}

pub fn collect_items_for_floor(state: &mut TransferState, floor: u32) -> Vec<FutureTransferItem> {
    let mut result = Vec::new();
    for slot in &mut state.future_items {
        if let Some(item) = slot
            && item.target_floor == floor
        {
            result.push(*item);
            *slot = None;
        }
    }
    result
}

// --- Systems ---

#[allow(clippy::too_many_arguments)]
fn spawn_transferred_items(
    mut commands: Commands,
    mut transfer_state: ResMut<TransferState>,
    current_floor: Res<CurrentFloor>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    mut rng: ResMut<DungeonRng>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
) {
    let floor = current_floor.number();
    let tile_size = config.dungeon.tile_size;
    let sprite_size = tile_size * config.item.item_sprite_scale;

    // 未来送りアイテムのスポーン（レベルブースト適用）
    let future_items = collect_items_for_floor(&mut transfer_state, floor);
    for item in &future_items {
        let boosted_level = future_transfer_level(item.source_floor, item.target_floor);
        let mut boosted_spec = item.spec;
        boosted_spec.level = boosted_level;
        boosted_spec.value = recompute_item_value(&boosted_spec, &config.item);
        spawn_transfer_item(
            &mut commands,
            &floor_map,
            &mut rng,
            boosted_spec,
            tile_size,
            sprite_size,
            &sprite_assets,
            &images,
        );
    }

    // 過去送りアイテムは reset_for_new_run で future_items に変換済み
    // （未来送りと同じフローでスポーンされる）
}

#[allow(clippy::too_many_arguments)]
fn spawn_transfer_item(
    commands: &mut Commands,
    floor_map: &FloorMap,
    rng: &mut ResMut<DungeonRng>,
    spec: ItemSpec,
    tile_size: f32,
    sprite_size: f32,
    sprite_assets: &SpriteAssets,
    images: &Assets<Image>,
) {
    // ランダムな部屋にスポーン
    if floor_map.rooms.is_empty() {
        return;
    }
    let room_idx = rng.0.random_range(0..floor_map.rooms.len());
    let room = &floor_map.rooms[room_idx];
    let x = rng.0.random_range(room.x..(room.x + room.width));
    let y = rng.0.random_range(room.y..(room.y + room.height));

    let color = rarity_color(spec.rarity);

    commands.spawn((
        make_sprite(
            sprite_assets.item_handle(spec.kind),
            images,
            color,
            color,
            Vec2::splat(sprite_size),
        ),
        Transform::from_xyz(x as f32 * tile_size, y as f32 * tile_size, 0.5),
        Item,
        ItemType(spec.kind),
        ItemRarity(spec.rarity),
        ItemLevel(spec.level),
        DroppedItem,
    ));

    info!(
        "Transferred item spawned: {:?} Lv{} (value: {})",
        spec.kind, spec.level, spec.value
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::item::{ItemKind, Rarity};

    fn make_spec(kind: ItemKind, level: u32, value: u32) -> ItemSpec {
        ItemSpec {
            kind,
            rarity: Rarity::Common,
            level,
            value,
        }
    }

    #[test]
    fn test_future_transfer_level() {
        // Lv = D + (D - S): 複利的レベルブースト
        assert_eq!(future_transfer_level(5, 8), 11); // 8 + 3
        assert_eq!(future_transfer_level(8, 15), 22); // 15 + 7
        assert_eq!(future_transfer_level(15, 30), 45); // 30 + 15
    }

    #[test]
    fn test_future_transfer_level_compound() {
        // 5F→8F: Lv11, 再送 8F→15F: Lv22 (複利的にレベルが上がる)
        let lv1 = future_transfer_level(5, 8);
        assert_eq!(lv1, 11);
        let lv2 = future_transfer_level(8, 15);
        assert_eq!(lv2, 22);
    }

    #[test]
    fn test_future_transfer_level_same_floor() {
        // source == target のとき D + 0 = D (過去送りのレベルリセット用)
        assert_eq!(future_transfer_level(5, 5), 5);
    }

    #[test]
    fn test_minimum_target_floor() {
        let spec = make_spec(ItemKind::Weapon, 10, 50);
        assert_eq!(minimum_target_floor(&spec), 11);

        let spec2 = make_spec(ItemKind::Head, 1, 5);
        assert_eq!(minimum_target_floor(&spec2), 2);
    }

    #[test]
    fn test_can_transfer() {
        let state = TransferState::new(5);
        assert!(can_transfer(&state));

        let state = TransferState::new(0);
        assert!(!can_transfer(&state));
    }

    #[test]
    fn test_add_future_transfer() {
        let mut state = TransferState::new(5);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        assert!(add_future_transfer(&mut state, spec, 5, 10));
        assert_eq!(state.charges, 4);
        assert!(state.future_items[0].is_some());
        assert_eq!(state.future_items[0].unwrap().target_floor, 10);
    }

    #[test]
    fn test_add_future_transfer_full() {
        let mut state = TransferState::new(100);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        // Fill all 32 slots
        for _ in 0..32 {
            assert!(add_future_transfer(&mut state, spec, 5, 10));
        }
        // 33rd should fail
        assert!(!add_future_transfer(&mut state, spec, 5, 10));
    }

    #[test]
    fn test_add_past_transfer() {
        let mut state = TransferState::new(5);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        assert!(add_past_transfer(&mut state, spec, 5));
        assert!(state.past_items[0].is_some());
        assert_eq!(state.past_items[0].unwrap().source_floor, 5);
        assert_eq!(state.charges, 4); // charges が内部で減算される
    }

    #[test]
    fn test_add_past_transfer_no_charges() {
        let mut state = TransferState::new(0);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        assert!(!add_past_transfer(&mut state, spec, 5));
    }

    #[test]
    fn test_add_past_transfer_full() {
        let mut state = TransferState::new(100);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        for _ in 0..32 {
            assert!(add_past_transfer(&mut state, spec, 5));
        }
        // スロット満杯
        assert!(!add_past_transfer(&mut state, spec, 5));
    }

    #[test]
    fn test_add_ending_carryover_no_charges() {
        // Ending 持ち帰りは charges を消費しない
        let mut state = TransferState::new(0);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        assert!(add_ending_carryover(&mut state, spec));
        assert_eq!(state.charges, 0);
        assert!(state.past_items[0].is_some());
        assert_eq!(state.past_items[0].unwrap().source_floor, 1);
    }

    #[test]
    fn test_collect_items_for_floor() {
        let mut state = TransferState::new(5);
        let spec1 = make_spec(ItemKind::Weapon, 5, 10);
        let spec2 = make_spec(ItemKind::Head, 5, 5);
        let spec3 = make_spec(ItemKind::Torso, 10, 20);

        add_future_transfer(&mut state, spec1, 5, 10);
        add_future_transfer(&mut state, spec2, 5, 10);
        add_future_transfer(&mut state, spec3, 5, 15);

        let items = collect_items_for_floor(&mut state, 10);
        assert_eq!(items.len(), 2);
        // Floor 15 item should remain
        assert!(state.future_items.iter().any(|s| s.is_some()));
    }

    #[test]
    fn test_collect_items_for_floor_empty() {
        let mut state = TransferState::new(5);
        let items = collect_items_for_floor(&mut state, 10);
        assert!(items.is_empty());
    }

    #[test]
    fn test_reset_for_new_run() {
        let mut state = TransferState::new(5);
        let spec = make_spec(ItemKind::Weapon, 10, 50);
        add_past_transfer(&mut state, spec, 8); // 8F から過去送り
        add_future_transfer(&mut state, spec, 5, 10); // 未来送りアイテム

        let mut rng = rand::rng();
        state.reset_for_new_run(5, &mut rng);

        assert_eq!(state.charges, 5);
        // past_items はクリアされ future_items に変換されている
        assert!(state.past_items.iter().all(|s| s.is_none()));
        // future_items に変換されている（前周回の未来アイテムはクリア済み）
        let converted: Vec<_> = state.future_items.iter().filter_map(|s| *s).collect();
        assert_eq!(converted.len(), 1);
        // スポーン階 = 1〜7 のランダム
        let spawn_floor = converted[0].target_floor;
        assert!(spawn_floor >= 1 && spawn_floor <= 7);
        // レベルがスポーン階にリセットされている
        assert_eq!(converted[0].spec.level, spawn_floor);
        // source_floor == target_floor (レベルブーストなし)
        assert_eq!(converted[0].source_floor, converted[0].target_floor);
    }

    #[test]
    fn test_reset_for_new_run_preserves_all_items() {
        let mut state = TransferState::new(100);
        let spec = make_spec(ItemKind::Weapon, 5, 10);
        // 5 個の past_items を追加
        for _ in 0..5 {
            add_past_transfer(&mut state, spec, 10);
        }
        let mut rng = rand::rng();
        state.reset_for_new_run(5, &mut rng);

        let converted_count = state.future_items.iter().filter(|s| s.is_some()).count();
        assert_eq!(converted_count, 5);
        assert!(state.past_items.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_past_transfer_from_floor_1() {
        // 1F からの過去送り: spawn_floor = 1
        let mut state = TransferState::new(5);
        let spec = make_spec(ItemKind::Weapon, 1, 5);
        add_past_transfer(&mut state, spec, 1);

        let mut rng = rand::rng();
        state.reset_for_new_run(5, &mut rng);

        let converted: Vec<_> = state.future_items.iter().filter_map(|s| *s).collect();
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].target_floor, 1);
        assert_eq!(converted[0].spec.level, 1);
    }
}
