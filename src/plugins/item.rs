use bevy::prelude::*;

use crate::components::item::{
    DroppedItem, EquipSlot, Item, ItemKind, ItemLevel, ItemRarity, ItemSpec, ItemType, Rarity,
};
use crate::components::{Attack, Defense, Health, Player};
use crate::config::{GameConfig, ItemConfig};
use crate::events::{EnemyDeathMessage, EnemyEquipmentDropMessage};
use crate::resources::player_state::{Equipment, Inventory, PlayerState};
use crate::resources::sprite_assets::{SpriteAssets, make_sprite};
use crate::resources::{ActiveCharmEffects, DungeonRng};
use crate::states::{GameState, PlayingSet};
use rand::Rng;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_equipment_drops,
                spawn_item_drop,
                item_pickup,
                update_player_stats,
            )
                .chain()
                .in_set(PlayingSet::Item)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            sync_charm_effects
                .in_set(PlayingSet::PostCombat)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// --- Charm Effects ---

#[derive(Debug, Clone, Copy, Default)]
pub struct CharmEffects {
    pub regen_interval: f32,
    pub regen_amount: u32,
    pub drop_bonus: f32,
    pub detection_reduction: f32,
    pub cooldown_reduction: f32,
}

pub fn charm_effects(charm: Option<&ItemSpec>) -> CharmEffects {
    let Some(charm) = charm else {
        return CharmEffects::default();
    };
    if charm.kind != ItemKind::Charm {
        return CharmEffects::default();
    }
    match charm.rarity {
        Rarity::Common => CharmEffects {
            regen_interval: 5.0,
            regen_amount: 1,
            ..Default::default()
        },
        Rarity::Uncommon => CharmEffects {
            drop_bonus: 0.15,
            ..Default::default()
        },
        Rarity::Rare => CharmEffects {
            detection_reduction: 0.3,
            ..Default::default()
        },
        Rarity::Epic => CharmEffects {
            cooldown_reduction: 0.2,
            ..Default::default()
        },
        Rarity::Legendary => CharmEffects {
            regen_interval: 5.0,
            regen_amount: 1,
            drop_bonus: 0.15,
            detection_reduction: 0.3,
            cooldown_reduction: 0.2,
        },
    }
}

// --- 純粋関数 ---

pub fn rarity_multiplier(rarity: Rarity) -> f32 {
    match rarity {
        Rarity::Common => 1.0,
        Rarity::Uncommon => 1.5,
        Rarity::Rare => 2.0,
        Rarity::Epic => 3.0,
        Rarity::Legendary => 5.0,
    }
}

pub fn compute_stat_value(base: u32, rarity: Rarity, level: u32, scaling: f32) -> u32 {
    let mult = rarity_multiplier(rarity);
    let level_mult = 1.0 + level as f32 * scaling;
    (base as f32 * mult * level_mult) as u32
}

pub fn determine_rarity(floor: u32, roll: f32) -> Rarity {
    let (common, uncommon, rare, epic) = if floor <= 10 {
        (0.70, 0.25, 0.05, 0.0)
    } else if floor <= 25 {
        (0.40, 0.35, 0.20, 0.05)
    } else if floor <= 40 {
        (0.20, 0.30, 0.30, 0.15)
    } else {
        (0.10, 0.20, 0.30, 0.25)
    };

    if roll < common {
        Rarity::Common
    } else if roll < common + uncommon {
        Rarity::Uncommon
    } else if roll < common + uncommon + rare {
        Rarity::Rare
    } else if roll < common + uncommon + rare + epic {
        Rarity::Epic
    } else {
        Rarity::Legendary
    }
}

pub fn determine_item_kind(roll: f32) -> ItemKind {
    // Weapon 12.5%, Head 12.5%, Torso 12.5%, Legs 12.5%,
    // Shield 12.5%, Charm 6.25%, Backpack 6.25%, HealthPotion 25%
    if roll < 0.125 {
        ItemKind::Weapon
    } else if roll < 0.25 {
        ItemKind::Head
    } else if roll < 0.375 {
        ItemKind::Torso
    } else if roll < 0.5 {
        ItemKind::Legs
    } else if roll < 0.625 {
        ItemKind::Shield
    } else if roll < 0.6875 {
        ItemKind::Charm
    } else if roll < 0.75 {
        ItemKind::Backpack
    } else {
        ItemKind::HealthPotion
    }
}

pub fn effective_attack(base: u32, equipment: &Equipment, config: &ItemConfig) -> u32 {
    let weapon_bonus = equipment.weapon.as_ref().map_or(0, |w| {
        compute_stat_value(
            config.weapon_base_stat,
            w.rarity,
            w.level,
            config.stat_level_scaling,
        )
    });
    base + weapon_bonus
}

pub fn effective_defense(base: u32, equipment: &Equipment, config: &ItemConfig) -> u32 {
    let defense_slots = [
        &equipment.head,
        &equipment.torso,
        &equipment.legs,
        &equipment.shield,
    ];
    let total_bonus: u32 = defense_slots
        .iter()
        .filter_map(|slot| slot.as_ref())
        .map(|spec| {
            compute_stat_value(
                config.armor_base_stat,
                spec.rarity,
                spec.level,
                config.stat_level_scaling,
            )
        })
        .sum();
    base + total_bonus
}

pub fn inventory_capacity(equipment: &Equipment, config: &ItemConfig) -> u8 {
    let base = 8u8;
    let backpack_bonus = equipment.backpack.as_ref().map_or(0u8, |bp| {
        let rarity_idx = match bp.rarity {
            Rarity::Common => 0,
            Rarity::Uncommon => 1,
            Rarity::Rare => 2,
            Rarity::Epic => 3,
            Rarity::Legendary => 4,
        };
        config.backpack_base_capacity + rarity_idx * config.backpack_capacity_per_rarity
    });
    base.saturating_add(backpack_bonus).min(16)
}

pub fn should_auto_equip(current: Option<&ItemSpec>, new_spec: &ItemSpec) -> bool {
    match current {
        None => true,
        Some(cur) => new_spec.value > cur.value,
    }
}

pub fn try_add_to_inventory(inventory: &mut Inventory, item: ItemSpec) -> bool {
    let cap = inventory.capacity as usize;
    for slot in &mut inventory.slots[..cap] {
        if slot.is_none() {
            *slot = Some(item);
            return true;
        }
    }
    false
}

pub fn recompute_item_value(spec: &ItemSpec, config: &ItemConfig) -> u32 {
    let base = match spec.kind {
        ItemKind::Weapon => config.weapon_base_stat,
        ItemKind::HealthPotion => config.potion_base_heal,
        _ => config.armor_base_stat,
    };
    compute_stat_value(base, spec.rarity, spec.level, config.stat_level_scaling)
}

pub fn item_kind_label(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Weapon => "武器",
        ItemKind::Head => "頭",
        ItemKind::Torso => "胴",
        ItemKind::Legs => "足",
        ItemKind::Shield => "盾",
        ItemKind::Charm => "装飾",
        ItemKind::Backpack => "背嚢",
        ItemKind::HealthPotion => "回復薬",
    }
}

pub fn equip_slot_label(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::Weapon => "武器",
        EquipSlot::Head => "頭",
        EquipSlot::Torso => "胴",
        EquipSlot::Legs => "足",
        EquipSlot::Shield => "盾",
        EquipSlot::Charm => "装飾",
        EquipSlot::Backpack => "背嚢",
    }
}

pub fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::Common => Color::srgb(0.8, 0.8, 0.8),
        Rarity::Uncommon => Color::srgb(0.2, 0.8, 0.2),
        Rarity::Rare => Color::srgb(0.3, 0.5, 1.0),
        Rarity::Epic => Color::srgb(0.7, 0.3, 0.9),
        Rarity::Legendary => Color::srgb(1.0, 0.85, 0.0),
    }
}

pub fn rarity_prefix(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Common => "",
        Rarity::Uncommon => "[U]",
        Rarity::Rare => "[R]",
        Rarity::Epic => "[E]",
        Rarity::Legendary => "[L]",
    }
}

// --- 敵装備関連 純粋関数 ---

/// レアリティの順序値（0-4）
pub fn rarity_rank(rarity: Rarity) -> u8 {
    match rarity {
        Rarity::Common => 0,
        Rarity::Uncommon => 1,
        Rarity::Rare => 2,
        Rarity::Epic => 3,
        Rarity::Legendary => 4,
    }
}

/// 装備スロット配列内の最高レアリティを返す
pub fn highest_rarity(slots: &[Option<ItemSpec>; 3]) -> Option<Rarity> {
    slots
        .iter()
        .filter_map(|s| s.as_ref())
        .max_by_key(|spec| rarity_rank(spec.rarity))
        .map(|spec| spec.rarity)
}

/// 敵装備による ATK ボーナス（Weapon のみ加算）
pub fn enemy_equipment_attack_bonus(slots: &[Option<ItemSpec>; 3], config: &ItemConfig) -> u32 {
    slots
        .iter()
        .filter_map(|s| s.as_ref())
        .filter(|spec| spec.kind == ItemKind::Weapon)
        .map(|spec| {
            compute_stat_value(
                config.weapon_base_stat,
                spec.rarity,
                spec.level,
                config.stat_level_scaling,
            )
        })
        .sum()
}

/// 敵装備による DEF ボーナス（防具のみ加算）
pub fn enemy_equipment_defense_bonus(slots: &[Option<ItemSpec>; 3], config: &ItemConfig) -> u32 {
    slots
        .iter()
        .filter_map(|s| s.as_ref())
        .filter(|spec| {
            matches!(
                spec.kind,
                ItemKind::Head | ItemKind::Torso | ItemKind::Legs | ItemKind::Shield
            )
        })
        .map(|spec| {
            compute_stat_value(
                config.armor_base_stat,
                spec.rarity,
                spec.level,
                config.stat_level_scaling,
            )
        })
        .sum()
}

// --- Systems ---

fn spawn_equipment_drops(
    mut commands: Commands,
    mut events: MessageReader<EnemyEquipmentDropMessage>,
    config: Res<GameConfig>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
) {
    for event in events.read() {
        let tile_size = config.dungeon.tile_size;
        let sprite_size = tile_size * config.item.item_sprite_scale;

        let mut offset_idx: f32 = 0.0;
        for slot in &event.items {
            let Some(spec) = slot else {
                continue;
            };
            let offset = Vec2::new(
                offset_idx * sprite_size * 0.6,
                offset_idx * sprite_size * 0.3,
            );
            let color = rarity_color(spec.rarity);

            commands.spawn((
                make_sprite(
                    sprite_assets.item_handle(spec.kind),
                    &images,
                    color,
                    color,
                    Vec2::splat(sprite_size),
                ),
                Transform::from_xyz(
                    event.position.x + offset.x,
                    event.position.y + offset.y,
                    0.5,
                ),
                Item,
                ItemType(spec.kind),
                ItemRarity(spec.rarity),
                ItemLevel(spec.level),
                DroppedItem,
            ));

            info!(
                "Enemy equipment dropped: {}{:?} Lv{}",
                rarity_prefix(spec.rarity),
                spec.kind,
                spec.level
            );
            offset_idx += 1.0;
        }
    }
}

fn spawn_item_drop(
    mut commands: Commands,
    mut events: MessageReader<EnemyDeathMessage>,
    mut rng: ResMut<DungeonRng>,
    config: Res<GameConfig>,
    effects: Res<ActiveCharmEffects>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
) {
    for event in events.read() {
        let drop_roll: f32 = rng.0.random();
        let effective_drop_rate = config.item.drop_rate + effects.0.drop_bonus;
        if drop_roll >= effective_drop_rate {
            continue;
        }

        let kind_roll: f32 = rng.0.random();
        let kind = determine_item_kind(kind_roll);

        let rarity_roll: f32 = rng.0.random();
        let rarity = determine_rarity(event.floor, rarity_roll);
        let level = event.floor;

        let base = match kind {
            ItemKind::Weapon => config.item.weapon_base_stat,
            ItemKind::HealthPotion => config.item.potion_base_heal,
            _ => config.item.armor_base_stat,
        };
        let value = compute_stat_value(base, rarity, level, config.item.stat_level_scaling);

        let tile_size = config.dungeon.tile_size;
        let sprite_size = tile_size * config.item.item_sprite_scale;
        let color = rarity_color(rarity);

        commands.spawn((
            make_sprite(
                sprite_assets.item_handle(kind),
                &images,
                color,
                color,
                Vec2::splat(sprite_size),
            ),
            Transform::from_xyz(event.position.x, event.position.y, 0.5),
            Item,
            ItemType(kind),
            ItemRarity(rarity),
            ItemLevel(level),
            DroppedItem,
        ));

        info!(
            "Item dropped: {}{:?} Lv{} (value: {})",
            rarity_prefix(rarity),
            kind,
            level,
            value
        );
    }
}

fn item_pickup(
    mut commands: Commands,
    mut player_query: Query<(&Transform, &mut Health), With<Player>>,
    item_query: Query<(Entity, &Transform, &ItemType, &ItemRarity, &ItemLevel), With<DroppedItem>>,
    mut player_state: ResMut<PlayerState>,
    config: Res<GameConfig>,
) {
    let Ok((player_transform, mut health)) = player_query.single_mut() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();
    let pickup_distance = config.dungeon.tile_size * 0.6;

    // インベントリ容量を装備から計算
    let new_capacity = inventory_capacity(&player_state.equipment, &config.item);
    if player_state.inventory.capacity != new_capacity {
        player_state.inventory.capacity = new_capacity;
    }

    for (entity, item_transform, item_type, item_rarity, item_level) in &item_query {
        let item_pos = item_transform.translation.truncate();
        let distance = player_pos.distance(item_pos);

        if distance > pickup_distance {
            continue;
        }

        let base = match item_type.0 {
            ItemKind::Weapon => config.item.weapon_base_stat,
            ItemKind::HealthPotion => config.item.potion_base_heal,
            _ => config.item.armor_base_stat,
        };
        let value = compute_stat_value(
            base,
            item_rarity.0,
            item_level.0,
            config.item.stat_level_scaling,
        );

        let spec = ItemSpec {
            kind: item_type.0,
            rarity: item_rarity.0,
            level: item_level.0,
            value,
        };

        match item_type.0 {
            ItemKind::HealthPotion => {
                let old_hp = health.current;
                health.current = (health.current + value).min(health.max);
                info!(
                    "Picked up HealthPotion! HP: {} -> {}",
                    old_hp, health.current
                );
            }
            _ => {
                if let Some(slot) = item_type.0.equip_slot() {
                    let current = player_state.equipment.get(slot);
                    if should_auto_equip(current, &spec) {
                        let old = player_state.equipment.set(slot, Some(spec));
                        info!(
                            "Equipped {}:{:?}(+{})!",
                            rarity_prefix(spec.rarity),
                            spec.kind,
                            spec.value
                        );
                        if let Some(old_spec) = old
                            && !try_add_to_inventory(&mut player_state.inventory, old_spec)
                        {
                            info!(
                                "Inventory full, discarded old {:?}(+{})",
                                old_spec.kind, old_spec.value
                            );
                        }
                    } else if !try_add_to_inventory(&mut player_state.inventory, spec) {
                        info!("Inventory full, discarded {:?}(+{})", spec.kind, spec.value);
                    }
                }
            }
        }

        commands.entity(entity).despawn();
    }
}

fn update_player_stats(
    mut query: Query<(&mut Attack, &mut Defense), With<Player>>,
    player_state: Res<PlayerState>,
    config: Res<GameConfig>,
) {
    let Ok((mut attack, mut defense)) = query.single_mut() else {
        return;
    };

    attack.0 = effective_attack(config.player.attack, &player_state.equipment, &config.item);
    defense.0 = effective_defense(config.player.defense, &player_state.equipment, &config.item);
}

fn sync_charm_effects(
    player_state: Res<PlayerState>,
    mut active_effects: ResMut<ActiveCharmEffects>,
) {
    if !player_state.is_changed() {
        return;
    }
    active_effects.0 = charm_effects(player_state.equipment.charm.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_item_config() -> ItemConfig {
        ItemConfig {
            drop_rate: 0.30,
            weapon_base_stat: 5,
            armor_base_stat: 3,
            potion_base_heal: 20,
            stat_level_scaling: 0.1,
            item_sprite_scale: 0.5,
            backpack_base_capacity: 2,
            backpack_capacity_per_rarity: 1,
        }
    }

    fn make_spec(kind: ItemKind, rarity: Rarity, level: u32, value: u32) -> ItemSpec {
        ItemSpec {
            kind,
            rarity,
            level,
            value,
        }
    }

    #[test]
    fn test_rarity_multiplier() {
        assert_eq!(rarity_multiplier(Rarity::Common), 1.0);
        assert_eq!(rarity_multiplier(Rarity::Uncommon), 1.5);
        assert_eq!(rarity_multiplier(Rarity::Rare), 2.0);
        assert_eq!(rarity_multiplier(Rarity::Epic), 3.0);
        assert_eq!(rarity_multiplier(Rarity::Legendary), 5.0);
    }

    #[test]
    fn test_compute_stat_value() {
        // base=5, Common, level=1, scaling=0.1 → 5 * 1.0 * 1.1 = 5
        assert_eq!(compute_stat_value(5, Rarity::Common, 1, 0.1), 5);
        // base=5, Rare, level=10, scaling=0.1 → 5 * 2.0 * 2.0 = 20
        assert_eq!(compute_stat_value(5, Rarity::Rare, 10, 0.1), 20);
        // base=5, Legendary, level=50, scaling=0.1 → 5 * 5.0 * 6.0 = 150
        assert_eq!(compute_stat_value(5, Rarity::Legendary, 50, 0.1), 150);
    }

    #[test]
    fn test_determine_rarity_floor_boundaries() {
        // Floor 1-10: 70% common
        assert_eq!(determine_rarity(1, 0.0), Rarity::Common);
        assert_eq!(determine_rarity(10, 0.69), Rarity::Common);
        assert_eq!(determine_rarity(10, 0.71), Rarity::Uncommon);
        assert_eq!(determine_rarity(10, 0.96), Rarity::Rare);

        // Floor 11-25: 40% common
        assert_eq!(determine_rarity(11, 0.0), Rarity::Common);
        assert_eq!(determine_rarity(25, 0.41), Rarity::Uncommon);
        assert_eq!(determine_rarity(25, 0.76), Rarity::Rare);
        assert_eq!(determine_rarity(25, 0.96), Rarity::Epic);

        // Floor 26-40
        assert_eq!(determine_rarity(26, 0.0), Rarity::Common);
        assert_eq!(determine_rarity(40, 0.81), Rarity::Epic);
        assert_eq!(determine_rarity(40, 0.96), Rarity::Legendary);

        // Floor 41+
        assert_eq!(determine_rarity(41, 0.0), Rarity::Common);
        assert_eq!(determine_rarity(50, 0.86), Rarity::Legendary);
    }

    #[test]
    fn test_determine_item_kind() {
        assert_eq!(determine_item_kind(0.0), ItemKind::Weapon);
        assert_eq!(determine_item_kind(0.12), ItemKind::Weapon);
        assert_eq!(determine_item_kind(0.13), ItemKind::Head);
        assert_eq!(determine_item_kind(0.26), ItemKind::Torso);
        assert_eq!(determine_item_kind(0.38), ItemKind::Legs);
        assert_eq!(determine_item_kind(0.51), ItemKind::Shield);
        assert_eq!(determine_item_kind(0.63), ItemKind::Charm);
        assert_eq!(determine_item_kind(0.69), ItemKind::Backpack);
        assert_eq!(determine_item_kind(0.76), ItemKind::HealthPotion);
        assert_eq!(determine_item_kind(0.99), ItemKind::HealthPotion);
    }

    #[test]
    fn test_effective_attack_with_weapon() {
        let config = default_item_config();
        let equipment = Equipment {
            weapon: Some(make_spec(ItemKind::Weapon, Rarity::Rare, 10, 20)),
            ..Default::default()
        };
        // base 20 + compute_stat_value(5, Rare, 10, 0.1) = 20 + 20 = 40
        assert_eq!(effective_attack(20, &equipment, &config), 40);
    }

    #[test]
    fn test_effective_attack_no_weapon() {
        let config = default_item_config();
        let equipment = Equipment::default();
        assert_eq!(effective_attack(20, &equipment, &config), 20);
    }

    #[test]
    fn test_effective_defense_full_armor() {
        let config = default_item_config();
        let equipment = Equipment {
            head: Some(make_spec(ItemKind::Head, Rarity::Common, 1, 3)),
            torso: Some(make_spec(ItemKind::Torso, Rarity::Common, 1, 3)),
            legs: Some(make_spec(ItemKind::Legs, Rarity::Common, 1, 3)),
            shield: Some(make_spec(ItemKind::Shield, Rarity::Common, 1, 3)),
            ..Default::default()
        };
        // base 5 + 4 * compute_stat_value(3, Common, 1, 0.1) = 5 + 4*3 = 17
        assert_eq!(effective_defense(5, &equipment, &config), 17);
    }

    #[test]
    fn test_inventory_capacity_with_backpack() {
        let config = default_item_config();
        // No backpack: capacity = 8
        let equipment = Equipment::default();
        assert_eq!(inventory_capacity(&equipment, &config), 8);

        // Common backpack: 8 + 2 + 0*1 = 10
        let equipment = Equipment {
            backpack: Some(make_spec(ItemKind::Backpack, Rarity::Common, 1, 1)),
            ..Default::default()
        };
        assert_eq!(inventory_capacity(&equipment, &config), 10);

        // Legendary backpack: 8 + 2 + 4*1 = 14
        let equipment = Equipment {
            backpack: Some(make_spec(ItemKind::Backpack, Rarity::Legendary, 1, 1)),
            ..Default::default()
        };
        assert_eq!(inventory_capacity(&equipment, &config), 14);
    }

    #[test]
    fn test_should_auto_equip() {
        let spec = make_spec(ItemKind::Weapon, Rarity::Common, 1, 10);
        // No current → equip
        assert!(should_auto_equip(None, &spec));

        // Current < new → equip
        let current = make_spec(ItemKind::Weapon, Rarity::Common, 1, 5);
        assert!(should_auto_equip(Some(&current), &spec));

        // Current >= new → don't equip
        let current = make_spec(ItemKind::Weapon, Rarity::Rare, 5, 15);
        assert!(!should_auto_equip(Some(&current), &spec));
    }

    #[test]
    fn test_try_add_to_inventory_respects_capacity() {
        let mut inventory = Inventory {
            slots: [None; 16],
            capacity: 8,
        };

        // Fill up to capacity
        for i in 0..8 {
            let item = make_spec(ItemKind::Weapon, Rarity::Common, 1, i + 1);
            assert!(try_add_to_inventory(&mut inventory, item));
        }

        // 9th item should fail (capacity = 8)
        let item = make_spec(ItemKind::Weapon, Rarity::Common, 1, 100);
        assert!(!try_add_to_inventory(&mut inventory, item));

        // But there's still physical space in slots[8..16]
        assert!(inventory.slots[8].is_none());
    }

    #[test]
    fn test_inventory_add_item() {
        let mut inventory = Inventory::default();
        let item = make_spec(ItemKind::Weapon, Rarity::Common, 1, 5);
        assert!(try_add_to_inventory(&mut inventory, item));
        assert!(inventory.slots[0].is_some());
        assert_eq!(inventory.slots[0].unwrap().value, 5);
    }

    #[test]
    fn test_recompute_item_value() {
        let config = default_item_config();

        // Weapon: base=5, Common, level=10, scaling=0.1 → 5 * 1.0 * 2.0 = 10
        let spec = make_spec(ItemKind::Weapon, Rarity::Common, 10, 0);
        assert_eq!(recompute_item_value(&spec, &config), 10);

        // Armor(Head): base=3, Rare, level=11, scaling=0.1 → 3 * 2.0 * 2.1 = 12
        let spec = make_spec(ItemKind::Head, Rarity::Rare, 11, 0);
        assert_eq!(recompute_item_value(&spec, &config), 12);

        // HealthPotion: base=20, Common, level=5, scaling=0.1 → 20 * 1.0 * 1.5 = 30
        let spec = make_spec(ItemKind::HealthPotion, Rarity::Common, 5, 0);
        assert_eq!(recompute_item_value(&spec, &config), 30);
    }

    #[test]
    fn test_charm_effects_common() {
        let spec = ItemSpec {
            kind: ItemKind::Charm,
            rarity: Rarity::Common,
            level: 1,
            value: 1,
        };
        let effects = charm_effects(Some(&spec));
        assert!(effects.regen_interval > 0.0);
        assert_eq!(effects.regen_amount, 1);
        assert_eq!(effects.drop_bonus, 0.0);
        assert_eq!(effects.detection_reduction, 0.0);
        assert_eq!(effects.cooldown_reduction, 0.0);
    }

    #[test]
    fn test_charm_effects_legendary() {
        let spec = ItemSpec {
            kind: ItemKind::Charm,
            rarity: Rarity::Legendary,
            level: 1,
            value: 1,
        };
        let effects = charm_effects(Some(&spec));
        assert!(effects.regen_interval > 0.0);
        assert_eq!(effects.regen_amount, 1);
        assert!(effects.drop_bonus > 0.0);
        assert!(effects.detection_reduction > 0.0);
        assert!(effects.cooldown_reduction > 0.0);
    }

    #[test]
    fn test_charm_effects_none() {
        let effects = charm_effects(None);
        assert_eq!(effects.regen_interval, 0.0);
        assert_eq!(effects.regen_amount, 0);
        assert_eq!(effects.drop_bonus, 0.0);
        assert_eq!(effects.detection_reduction, 0.0);
        assert_eq!(effects.cooldown_reduction, 0.0);
    }

    #[test]
    fn test_rarity_rank_ordering() {
        assert_eq!(rarity_rank(Rarity::Common), 0);
        assert_eq!(rarity_rank(Rarity::Uncommon), 1);
        assert_eq!(rarity_rank(Rarity::Rare), 2);
        assert_eq!(rarity_rank(Rarity::Epic), 3);
        assert_eq!(rarity_rank(Rarity::Legendary), 4);
        assert!(rarity_rank(Rarity::Common) < rarity_rank(Rarity::Legendary));
    }

    #[test]
    fn test_highest_rarity_empty() {
        let slots: [Option<ItemSpec>; 3] = [None, None, None];
        assert!(highest_rarity(&slots).is_none());
    }

    #[test]
    fn test_highest_rarity_mixed() {
        let slots: [Option<ItemSpec>; 3] = [
            Some(make_spec(ItemKind::Weapon, Rarity::Common, 1, 5)),
            Some(make_spec(ItemKind::Head, Rarity::Epic, 1, 10)),
            Some(make_spec(ItemKind::Torso, Rarity::Rare, 1, 8)),
        ];
        assert_eq!(highest_rarity(&slots), Some(Rarity::Epic));
    }

    #[test]
    fn test_enemy_equipment_attack_bonus_weapon() {
        let config = default_item_config();
        let slots: [Option<ItemSpec>; 3] = [
            Some(make_spec(ItemKind::Weapon, Rarity::Common, 10, 0)),
            None,
            None,
        ];
        // compute_stat_value(5, Common, 10, 0.1) = 5 * 1.0 * 2.0 = 10
        assert_eq!(enemy_equipment_attack_bonus(&slots, &config), 10);
    }

    #[test]
    fn test_enemy_equipment_attack_bonus_armor_only() {
        let config = default_item_config();
        let slots: [Option<ItemSpec>; 3] = [
            Some(make_spec(ItemKind::Head, Rarity::Common, 10, 0)),
            Some(make_spec(ItemKind::Torso, Rarity::Rare, 5, 0)),
            None,
        ];
        // 防具は ATK 0
        assert_eq!(enemy_equipment_attack_bonus(&slots, &config), 0);
    }

    #[test]
    fn test_enemy_equipment_defense_bonus() {
        let config = default_item_config();
        let slots: [Option<ItemSpec>; 3] = [
            Some(make_spec(ItemKind::Head, Rarity::Common, 10, 0)),
            Some(make_spec(ItemKind::Torso, Rarity::Common, 10, 0)),
            Some(make_spec(ItemKind::Shield, Rarity::Common, 10, 0)),
        ];
        // compute_stat_value(3, Common, 10, 0.1) = 3 * 1.0 * 2.0 = 6 each → 18
        assert_eq!(enemy_equipment_defense_bonus(&slots, &config), 18);
    }

    #[test]
    fn test_enemy_equipment_defense_bonus_weapon_only() {
        let config = default_item_config();
        let slots: [Option<ItemSpec>; 3] = [
            Some(make_spec(ItemKind::Weapon, Rarity::Legendary, 50, 0)),
            None,
            None,
        ];
        // 武器は DEF 0
        assert_eq!(enemy_equipment_defense_bonus(&slots, &config), 0);
    }
}
