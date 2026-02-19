use bevy::prelude::*;

#[derive(Clone, Copy)]
pub struct DungeonConfig {
    pub max_floor: u32,
    pub tile_size: f32,
    pub map_width: i32,
    pub map_height: i32,
    pub min_room_size: i32,
    pub max_room_size: i32,
}

#[derive(Clone, Copy)]
pub struct PlayerConfig {
    pub speed: f32,
    pub hp: u32,
    pub attack: u32,
    pub defense: u32,
    pub attack_range: f32,
    pub attack_angle: f32,
    pub attack_cooldown: f32,
    pub invincibility: f32,
}

#[derive(Clone, Copy)]
pub struct CombatConfig {
    pub attack_effect_duration: f32,
}

#[derive(Clone, Copy)]
pub struct EnemyConfig {
    pub min_count: u32,
    pub max_count: u32,
    pub respawn_interval: f32,
    pub slime_base_hp: u32,
    pub slime_hp_per_floor: u32,
    pub slime_base_attack: u32,
    pub slime_attack_per_floor: u32,
    pub slime_base_defense: f32,
    pub slime_defense_per_floor: f32,
    pub slime_base_speed: f32,
    pub slime_speed_per_floor: f32,
    pub slime_detection_radius: f32,
    pub slime_attack_range: f32,
    pub slime_attack_cooldown: f32,
    pub slime_chase_lost_time: f32,
}

#[derive(Clone, Copy)]
pub struct ItemConfig {
    pub drop_rate: f32,
    pub weapon_base_stat: u32,
    pub armor_base_stat: u32,
    pub potion_base_heal: u32,
    pub stat_level_scaling: f32,
    pub item_sprite_scale: f32,
    pub backpack_base_capacity: u8,
    pub backpack_capacity_per_rarity: u8,
}

#[derive(Clone, Copy)]
pub struct TransferConfig {
    pub charges_per_run: u32,
}

#[derive(Resource)]
pub struct GameConfig {
    pub dungeon: DungeonConfig,
    pub player: PlayerConfig,
    pub combat: CombatConfig,
    pub enemy: EnemyConfig,
    pub item: ItemConfig,
    pub transfer: TransferConfig,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            dungeon: DungeonConfig {
                max_floor: 50,
                tile_size: 32.0,
                map_width: 48,
                map_height: 48,
                min_room_size: 5,
                max_room_size: 15,
            },
            player: PlayerConfig {
                speed: 150.0,
                hp: 100,
                attack: 20,
                defense: 5,
                attack_range: 48.0,
                attack_angle: 60.0,
                attack_cooldown: 0.4,
                invincibility: 0.5,
            },
            combat: CombatConfig {
                attack_effect_duration: 0.15,
            },
            enemy: EnemyConfig {
                min_count: 5,
                max_count: 10,
                respawn_interval: 15.0,
                slime_base_hp: 10,
                slime_hp_per_floor: 3,
                slime_base_attack: 20,
                slime_attack_per_floor: 1,
                slime_base_defense: 2.0,
                slime_defense_per_floor: 0.5,
                slime_base_speed: 60.0,
                slime_speed_per_floor: 1.0,
                slime_detection_radius: 6.0,
                slime_attack_range: 32.0,
                slime_attack_cooldown: 1.0,
                slime_chase_lost_time: 1.0,
            },
            item: ItemConfig {
                drop_rate: 0.30,
                weapon_base_stat: 5,
                armor_base_stat: 3,
                potion_base_heal: 20,
                stat_level_scaling: 0.1,
                item_sprite_scale: 0.5,
                backpack_base_capacity: 2,
                backpack_capacity_per_rarity: 1,
            },
            transfer: TransferConfig {
                charges_per_run: 5,
            },
        }
    }
}
