use bevy::prelude::*;

use crate::components::item::{DroppedItem, Item, ItemLevel, ItemRarity, ItemSpec, ItemType};
use crate::config::GameConfig;
use crate::plugins::item::{item_kind_label, rarity_color, rarity_prefix, recompute_item_value};
use crate::resources::font::GameFont;
use crate::resources::sprite_assets::{SpriteAssets, make_sprite};
use crate::resources::transfer_state::{
    ArrivedItemInfo, FutureTransferItem, PastTransferItem, TransferArrivalNotice, TransferState,
};
use crate::resources::{CurrentFloor, DungeonRng, FloorMap};
use crate::states::FloorTransitionSetup;
use crate::states::GameState;
use rand::Rng;

const BANNER_LIFETIME: f32 = 3.0;
const BANNER_FADE_DURATION: f32 = 1.0;

#[derive(Component)]
struct TransferBanner {
    lifetime: f32,
}

pub struct TransferPlugin;

impl Plugin for TransferPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::FloorTransition),
            spawn_transferred_items.in_set(FloorTransitionSetup::SpawnEntities),
        )
        .add_systems(OnEnter(GameState::Playing), spawn_transfer_banner)
        .add_systems(
            Update,
            update_transfer_banner.run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnEnter(GameState::GameOver), cleanup_banner)
        .add_systems(OnEnter(GameState::Ending), cleanup_banner);
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
    mut arrival_notice: ResMut<TransferArrivalNotice>,
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

    // 到着通知をクリアして新たに書き込み
    arrival_notice.clear();
    let mut notice_idx = 0;

    for item in &future_items {
        let boosted_level = future_transfer_level(item.source_floor, item.target_floor);
        let mut boosted_spec = item.spec;
        boosted_spec.level = boosted_level;
        boosted_spec.value = recompute_item_value(&boosted_spec, &config.item);

        // 到着通知に記録
        if notice_idx < arrival_notice.items.len() {
            arrival_notice.items[notice_idx] = Some(ArrivedItemInfo {
                kind: boosted_spec.kind,
                rarity: boosted_spec.rarity,
                level: boosted_spec.level,
            });
            notice_idx += 1;
        }

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

// --- バナーシステム ---

fn spawn_transfer_banner(
    mut commands: Commands,
    mut arrival_notice: ResMut<TransferArrivalNotice>,
    existing: Query<Entity, With<TransferBanner>>,
    game_font: Res<GameFont>,
) {
    // 既存バナーがあれば何もしない（InventoryOpen→Playing 復帰時の二重スポーン防止）
    if !existing.is_empty() {
        return;
    }

    if arrival_notice.is_empty() {
        return;
    }

    // アイテムリスト文字列を生成
    let mut parts: Vec<String> = Vec::new();
    for info in arrival_notice.items.iter().flatten() {
        let prefix = rarity_prefix(info.rarity);
        let label = item_kind_label(info.kind);
        if prefix.is_empty() {
            parts.push(format!("{} Lv{}", label, info.level));
        } else {
            parts.push(format!("{}{} Lv{}", prefix, label, info.level));
        }
    }
    let items_text = parts.join(", ");

    // notice をクリア（InventoryOpen→Playing 復帰時に再表示しない）
    arrival_notice.clear();

    let font = game_font.0.clone();

    commands
        .spawn((
            TransferBanner {
                lifetime: BANNER_LIFETIME,
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                left: Val::Percent(50.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.3, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("転送アイテムがこの階に到着!"),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
            ));
            parent.spawn((
                Text::new(items_text),
                TextFont {
                    font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn update_transfer_banner(
    mut commands: Commands,
    time: Res<Time>,
    mut banner_query: Query<(Entity, &mut TransferBanner, &mut BackgroundColor, &Children)>,
    mut text_color_query: Query<&mut TextColor>,
) {
    for (entity, mut banner, mut bg_color, children) in &mut banner_query {
        banner.lifetime -= time.delta_secs();

        if banner.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 残り BANNER_FADE_DURATION 秒でフェードアウト
        if banner.lifetime < BANNER_FADE_DURATION {
            let alpha = banner.lifetime / BANNER_FADE_DURATION;
            // 背景色のアルファを減衰
            let mut bg = bg_color.0.to_srgba();
            bg.alpha = 0.85 * alpha;
            bg_color.0 = bg.into();

            // 子テキストの TextColor アルファを減衰
            for child in children.iter() {
                if let Ok(mut text_color) = text_color_query.get_mut(child) {
                    let mut c = text_color.0.to_srgba();
                    c.alpha = alpha;
                    text_color.0 = c.into();
                }
            }
        }
    }
}

fn cleanup_banner(mut commands: Commands, query: Query<Entity, With<TransferBanner>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
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
