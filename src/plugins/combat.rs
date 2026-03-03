use bevy::prelude::*;

use crate::components::item::ItemKind;
use crate::components::{
    AttackCooldown, AttackEffect, Dead, Enemy, EnemyEquipment, Health, InvincibilityTimer, Player,
};
use crate::config::{EnemyConfig, GameConfig};
use crate::events::{DamageApplied, DamageEvent, EnemyDeathMessage, EnemyEquipmentDropMessage};
use crate::resources::CurrentFloor;
use crate::states::{GameState, PlayingSet};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    update_invincibility,
                    process_damage,
                    check_death,
                    update_damage_visual,
                )
                    .chain(),
                update_attack_cooldowns,
                update_attack_effects,
            )
                .in_set(PlayingSet::Combat)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            cleanup_dead
                .in_set(PlayingSet::PostCombat)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// --- 静的メタデータ ---

use crate::components::EnemyKind;

pub struct EnemyKindMeta {
    pub hp_mult: f32,
    pub attack_mult: f32,
    pub defense_mult: f32,
    pub speed_mult: f32,
    pub detection_mult: f32,
    pub attack_range_mult: f32,
    pub attack_cooldown: f32,
    pub wander_min: f32,
    pub wander_max: f32,
    pub color: (f32, f32, f32),
    pub equip_slots_min: u8,
    pub equip_slots_max: u8,
}

pub static ENEMY_KIND_META: &[EnemyKindMeta; 6] = &[
    // 0: Slime
    EnemyKindMeta {
        hp_mult: 1.0,
        attack_mult: 1.0,
        defense_mult: 1.0,
        speed_mult: 1.0,
        detection_mult: 1.0,
        attack_range_mult: 1.0,
        attack_cooldown: 1.0,
        wander_min: 2.0,
        wander_max: 4.0,
        color: (0.2, 0.8, 0.3),
        equip_slots_min: 0,
        equip_slots_max: 1,
    },
    // 1: Bat
    EnemyKindMeta {
        hp_mult: 0.5,
        attack_mult: 0.8,
        defense_mult: 0.3,
        speed_mult: 1.8,
        detection_mult: 1.5,
        attack_range_mult: 1.0,
        attack_cooldown: 0.7,
        wander_min: 0.5,
        wander_max: 1.5,
        color: (0.6, 0.2, 0.8),
        equip_slots_min: 0,
        equip_slots_max: 2,
    },
    // 2: Golem
    EnemyKindMeta {
        hp_mult: 3.0,
        attack_mult: 1.5,
        defense_mult: 2.5,
        speed_mult: 0.5,
        detection_mult: 0.8,
        attack_range_mult: 1.2,
        attack_cooldown: 2.0,
        wander_min: 4.0,
        wander_max: 8.0,
        color: (0.5, 0.5, 0.55),
        equip_slots_min: 1,
        equip_slots_max: 3,
    },
    // 3: SlimeII
    EnemyKindMeta {
        hp_mult: 2.0,
        attack_mult: 1.5,
        defense_mult: 1.5,
        speed_mult: 0.8,
        detection_mult: 1.0,
        attack_range_mult: 1.0,
        attack_cooldown: 0.8,
        wander_min: 1.5,
        wander_max: 3.0,
        color: (0.1, 0.5, 0.2),
        equip_slots_min: 1,
        equip_slots_max: 2,
    },
    // 4: BatII
    EnemyKindMeta {
        hp_mult: 0.8,
        attack_mult: 1.2,
        defense_mult: 0.5,
        speed_mult: 2.0,
        detection_mult: 1.8,
        attack_range_mult: 1.0,
        attack_cooldown: 0.5,
        wander_min: 0.3,
        wander_max: 1.0,
        color: (0.8, 0.1, 0.3),
        equip_slots_min: 1,
        equip_slots_max: 3,
    },
    // 5: GolemII
    EnemyKindMeta {
        hp_mult: 4.0,
        attack_mult: 2.0,
        defense_mult: 2.8,
        speed_mult: 0.4,
        detection_mult: 0.8,
        attack_range_mult: 1.3,
        attack_cooldown: 2.5,
        wander_min: 3.0,
        wander_max: 6.0,
        color: (0.3, 0.3, 0.7),
        equip_slots_min: 2,
        equip_slots_max: 3,
    },
];

impl EnemyKind {
    pub const fn meta_index(self) -> usize {
        match self {
            EnemyKind::Slime => 0,
            EnemyKind::Bat => 1,
            EnemyKind::Golem => 2,
            EnemyKind::SlimeII => 3,
            EnemyKind::BatII => 4,
            EnemyKind::GolemII => 5,
        }
    }
}

// --- 純粋関数 ---

/// ダメージ計算: attack.saturating_sub(defense).max(1)
pub fn calculate_damage(attack: u32, defense: u32) -> u32 {
    attack.saturating_sub(defense).max(1)
}

/// 扇形範囲内に target が含まれるかを判定
pub fn is_in_attack_fan(
    origin: Vec2,
    facing: Vec2,
    target: Vec2,
    range: f32,
    angle_degrees: f32,
) -> bool {
    let to_target = target - origin;
    let distance = to_target.length();

    if distance < f32::EPSILON {
        return false;
    }
    if distance > range {
        return false;
    }

    let facing_normalized = facing.normalize_or_zero();
    if facing_normalized == Vec2::ZERO {
        return false;
    }

    let to_target_normalized = to_target / distance;
    let dot = facing_normalized.dot(to_target_normalized);
    let half_angle_cos = (angle_degrees / 2.0).to_radians().cos();

    dot >= half_angle_cos
}

/// min_count..=max_count のランダム敵数
pub fn enemy_count_random(min_count: u32, max_count: u32, roll: f32) -> u32 {
    let range = max_count - min_count + 1;
    min_count + (roll * range as f32) as u32
}

/// スライムステータス計算: (hp, atk, def, speed)
pub fn slime_stats(floor: u32, config: &EnemyConfig) -> (u32, u32, u32, f32) {
    let hp = config.slime_base_hp + floor * config.slime_hp_per_floor;
    let atk = config.slime_base_attack + floor * config.slime_attack_per_floor;
    let def =
        (config.slime_base_defense + floor as f32 * config.slime_defense_per_floor).round() as u32;
    let speed = config.slime_base_speed + floor as f32 * config.slime_speed_per_floor;
    (hp, atk, def, speed)
}

/// 種別に応じたステータス計算（Slime基準 × 乗数）
pub fn enemy_stats(kind: EnemyKind, floor: u32, config: &EnemyConfig) -> (u32, u32, u32, f32) {
    let (base_hp, base_atk, base_def, base_speed) = slime_stats(floor, config);
    let meta = &ENEMY_KIND_META[kind.meta_index()];
    (
        (base_hp as f32 * meta.hp_mult) as u32,
        (base_atk as f32 * meta.attack_mult) as u32,
        (base_def as f32 * meta.defense_mult) as u32,
        base_speed * meta.speed_mult,
    )
}

/// フロアと乱数に基づいて敵種別を決定
pub fn determine_enemy_kind(floor: u32, roll: f32) -> EnemyKind {
    if floor >= 10 {
        if roll < 0.10 {
            EnemyKind::Slime
        } else if roll < 0.20 {
            EnemyKind::Bat
        } else if roll < 0.30 {
            EnemyKind::Golem
        } else if roll < 0.55 {
            EnemyKind::SlimeII
        } else if roll < 0.80 {
            EnemyKind::BatII
        } else {
            EnemyKind::GolemII
        }
    } else if floor >= 6 {
        if roll < 0.40 {
            EnemyKind::Slime
        } else if roll < 0.75 {
            EnemyKind::Bat
        } else {
            EnemyKind::Golem
        }
    } else if floor >= 3 {
        if roll < 0.60 {
            EnemyKind::Slime
        } else {
            EnemyKind::Bat
        }
    } else {
        EnemyKind::Slime
    }
}

/// フロア依存の敵装備保有確率
pub fn enemy_hold_probability(floor: u32) -> f32 {
    0.05 + ((floor as f32 - 1.0) / 49.0).clamp(0.0, 1.0) * 0.75
}

/// 確率枠がアイテムを持つかどうかの判定
pub fn should_enemy_hold_item(floor: u32, roll: f32) -> bool {
    roll < enemy_hold_probability(floor)
}

/// 敵が装備可能な5種から均等選択
pub fn enemy_equippable_item_kind(roll: f32) -> ItemKind {
    if roll < 0.2 {
        ItemKind::Weapon
    } else if roll < 0.4 {
        ItemKind::Head
    } else if roll < 0.6 {
        ItemKind::Torso
    } else if roll < 0.8 {
        ItemKind::Legs
    } else {
        ItemKind::Shield
    }
}

// --- Systems ---

fn update_invincibility(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut InvincibilityTimer)>,
) {
    for (entity, mut timer) in &mut query {
        timer.remaining -= time.delta_secs();
        if timer.remaining <= 0.0 {
            commands.entity(entity).remove::<InvincibilityTimer>();
        }
    }
}

fn process_damage(
    mut commands: Commands,
    mut events: MessageReader<DamageEvent>,
    mut query: Query<(&mut Health, Has<InvincibilityTimer>, &Transform)>,
    source_query: Query<&Transform>,
    config: Res<GameConfig>,
    mut damage_applied: MessageWriter<DamageApplied>,
) {
    for event in events.read() {
        let Ok((mut health, has_invincibility, target_transform)) = query.get_mut(event.target)
        else {
            continue;
        };
        if health.current == 0 {
            continue;
        }
        if has_invincibility {
            continue;
        }
        health.current = health.current.saturating_sub(event.amount);
        let position = target_transform.translation.truncate();
        let source_position = source_query
            .get(event.source)
            .map(|t| t.translation.truncate())
            .unwrap_or(position);
        info!(
            "Damage: {} -> entity, HP: {}/{}",
            event.amount, health.current, health.max
        );
        commands.entity(event.target).insert(InvincibilityTimer {
            remaining: config.player.invincibility,
        });
        damage_applied.write(DamageApplied {
            target: event.target,
            amount: event.amount,
            position,
            source_position,
        });
    }
}

#[allow(clippy::type_complexity)]
fn check_death(
    mut commands: Commands,
    enemy_query: Query<
        (Entity, &Health, &Transform, Option<&EnemyEquipment>),
        (With<Enemy>, Without<Dead>),
    >,
    player_query: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut enemy_death_events: MessageWriter<EnemyDeathMessage>,
    mut equip_drop_events: MessageWriter<EnemyEquipmentDropMessage>,
    current_floor: Res<CurrentFloor>,
) {
    // 敵の死亡チェック
    for (entity, health, transform, equipment) in &enemy_query {
        if health.current == 0 {
            let pos = transform.translation.truncate();
            enemy_death_events.write(EnemyDeathMessage {
                position: pos,
                floor: current_floor.number(),
            });
            if let Some(equip) = equipment {
                equip_drop_events.write(EnemyEquipmentDropMessage {
                    position: pos,
                    floor: current_floor.number(),
                    items: equip.slots,
                });
            }
            info!("Enemy defeated!");
            commands.entity(entity).insert(Dead);
        }
    }

    // プレイヤーの死亡チェック
    for health in &player_query {
        if health.current == 0 {
            info!("Player defeated! Game Over.");
            next_state.set(GameState::GameOver);
        }
    }
}

fn cleanup_dead(mut commands: Commands, query: Query<Entity, With<Dead>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn update_attack_cooldowns(time: Res<Time>, mut query: Query<&mut AttackCooldown>) {
    for mut cooldown in &mut query {
        if cooldown.remaining > 0.0 {
            cooldown.remaining = (cooldown.remaining - time.delta_secs()).max(0.0);
        }
    }
}

fn update_attack_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AttackEffect, &mut Transform, &mut Sprite)>,
) {
    for (entity, mut effect, mut transform, mut sprite) in &mut query {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        let elapsed = effect.duration - effect.remaining;
        let t = (elapsed / effect.duration).clamp(0.0, 1.0);

        // 角度を線形補間
        let angle = effect.start_angle + (effect.end_angle - effect.start_angle) * t;
        transform.rotation = Quat::from_rotation_z(angle);

        // 後半 1/3 でフェードアウト（initial_alpha 基準）
        if t > 0.67 {
            let fade = ((1.0 - t) / 0.33).clamp(0.0, 1.0);
            sprite.color = sprite.color.with_alpha(effect.initial_alpha * fade);
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_damage_visual(
    mut query: Query<
        (&mut Sprite, Option<&InvincibilityTimer>),
        (With<Health>, Without<AttackEffect>),
    >,
) {
    for (mut sprite, invincibility) in &mut query {
        if invincibility.is_some() {
            // 無敵中は半透明 + 赤み
            sprite.color = sprite.color.with_alpha(0.5);
        } else {
            sprite.color = sprite.color.with_alpha(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EnemyConfig;

    #[test]
    fn test_damage_calculation_normal() {
        assert_eq!(calculate_damage(15, 5), 10);
    }

    #[test]
    fn test_damage_calculation_minimum() {
        assert_eq!(calculate_damage(3, 10), 1);
    }

    #[test]
    fn test_damage_calculation_zero_defense() {
        assert_eq!(calculate_damage(10, 0), 10);
    }

    #[test]
    fn test_fan_hit_inside() {
        let origin = Vec2::new(0.0, 0.0);
        let facing = Vec2::new(1.0, 0.0);
        let target = Vec2::new(30.0, 5.0);
        assert!(is_in_attack_fan(origin, facing, target, 48.0, 60.0));
    }

    #[test]
    fn test_fan_miss_too_far() {
        let origin = Vec2::new(0.0, 0.0);
        let facing = Vec2::new(1.0, 0.0);
        let target = Vec2::new(100.0, 0.0);
        assert!(!is_in_attack_fan(origin, facing, target, 48.0, 60.0));
    }

    #[test]
    fn test_fan_miss_angle() {
        let origin = Vec2::new(0.0, 0.0);
        let facing = Vec2::new(1.0, 0.0);
        // 90度横 → 60度の扇形からはみ出す
        let target = Vec2::new(0.0, 30.0);
        assert!(!is_in_attack_fan(origin, facing, target, 48.0, 60.0));
    }

    #[test]
    fn test_fan_boundary_angle() {
        let origin = Vec2::new(0.0, 0.0);
        let facing = Vec2::new(1.0, 0.0);
        // 半角 30 度のわずかに内側（29度）→ ヒット
        let angle_rad = 29.0_f32.to_radians();
        let target = Vec2::new(angle_rad.cos() * 40.0, angle_rad.sin() * 40.0);
        assert!(is_in_attack_fan(origin, facing, target, 48.0, 60.0));
        // 半角 30 度のわずかに外側（31度）→ ミス
        let angle_rad_out = 31.0_f32.to_radians();
        let target_out = Vec2::new(angle_rad_out.cos() * 40.0, angle_rad_out.sin() * 40.0);
        assert!(!is_in_attack_fan(origin, facing, target_out, 48.0, 60.0));
    }

    #[test]
    fn test_fan_same_position() {
        let origin = Vec2::new(10.0, 10.0);
        let facing = Vec2::new(1.0, 0.0);
        assert!(!is_in_attack_fan(origin, facing, origin, 48.0, 60.0));
    }

    #[test]
    fn test_enemy_count_random() {
        // roll=0.0 → min
        assert_eq!(enemy_count_random(5, 10, 0.0), 5);
        // roll=0.99 → max
        assert_eq!(enemy_count_random(5, 10, 0.99), 10);
        // roll=0.5 → mid
        assert_eq!(enemy_count_random(5, 10, 0.5), 8);
    }

    fn default_enemy_config() -> EnemyConfig {
        EnemyConfig {
            min_count: 5,
            max_count: 10,
            respawn_interval: 15.0,
            slime_base_hp: 10,
            slime_hp_per_floor: 3,
            slime_base_attack: 5,
            slime_attack_per_floor: 1,
            slime_base_defense: 2.0,
            slime_defense_per_floor: 0.5,
            slime_base_speed: 60.0,
            slime_speed_per_floor: 1.0,
            slime_detection_radius: 6.0,
            slime_attack_range: 32.0,
            slime_attack_cooldown: 1.0,
            slime_chase_lost_time: 1.0,
        }
    }

    #[test]
    fn test_slime_stats_scaling() {
        let config = default_enemy_config();

        // Floor 1: HP=13, ATK=6, DEF=round(2.5)=3, Speed=61
        let (hp, atk, def, speed) = slime_stats(1, &config);
        assert_eq!(hp, 13);
        assert_eq!(atk, 6);
        assert_eq!(def, 3); // round(2.5) = 3 (Rust rounds 0.5 to nearest even → 2, but 2.5f32.round() = 3.0)
        assert!((speed - 61.0).abs() < f32::EPSILON);

        // Floor 25: HP=85, ATK=30, DEF=round(14.5)=15, Speed=85
        let (hp, atk, def, speed) = slime_stats(25, &config);
        assert_eq!(hp, 85);
        assert_eq!(atk, 30);
        assert_eq!(def, 15); // round(14.5) = 15
        assert!((speed - 85.0).abs() < f32::EPSILON);

        // Floor 50: HP=160, ATK=55, DEF=round(27.0)=27, Speed=110
        let (hp, atk, def, speed) = slime_stats(50, &config);
        assert_eq!(hp, 160);
        assert_eq!(atk, 55);
        assert_eq!(def, 27);
        assert!((speed - 110.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_enemy_stats_bat() {
        let config = default_enemy_config();
        // Floor 5: slime base = (25, 10, round(4.5)=5, 65.0)
        // Bat: hp*0.5=12, atk*0.8=8, def*0.3=1, speed*1.8=117.0
        let (hp, atk, def, speed) = enemy_stats(EnemyKind::Bat, 5, &config);
        assert_eq!(hp, 12);
        assert_eq!(atk, 8);
        assert_eq!(def, 1);
        assert!((speed - 117.0).abs() < 0.1);
    }

    #[test]
    fn test_enemy_stats_golem() {
        let config = default_enemy_config();
        // Floor 15: slime base = (55, 20, round(9.5)=10, 75.0)
        // Golem: hp*3.0=165, atk*1.5=30, def*2.5=25, speed*0.5=37.5
        let (hp, atk, def, speed) = enemy_stats(EnemyKind::Golem, 15, &config);
        assert_eq!(hp, 165);
        assert_eq!(atk, 30);
        assert_eq!(def, 25);
        assert!((speed - 37.5).abs() < 0.1);
    }

    #[test]
    fn test_determine_enemy_kind_boundaries() {
        // Floor 1-2: always Slime
        assert_eq!(determine_enemy_kind(1, 0.0), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(2, 0.99), EnemyKind::Slime);

        // Floor 3-5: 60% Slime, 40% Bat
        assert_eq!(determine_enemy_kind(3, 0.0), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(3, 0.59), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(3, 0.61), EnemyKind::Bat);
        assert_eq!(determine_enemy_kind(5, 0.99), EnemyKind::Bat);

        // Floor 6-9: 40% Slime, 35% Bat, 25% Golem
        assert_eq!(determine_enemy_kind(6, 0.0), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(6, 0.39), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(6, 0.41), EnemyKind::Bat);
        assert_eq!(determine_enemy_kind(6, 0.74), EnemyKind::Bat);
        assert_eq!(determine_enemy_kind(6, 0.76), EnemyKind::Golem);
        assert_eq!(determine_enemy_kind(9, 0.99), EnemyKind::Golem);

        // Floor 10+: 10% Slime, 10% Bat, 10% Golem, 25% SlimeII, 25% BatII, 20% GolemII
        assert_eq!(determine_enemy_kind(10, 0.0), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(10, 0.09), EnemyKind::Slime);
        assert_eq!(determine_enemy_kind(10, 0.11), EnemyKind::Bat);
        assert_eq!(determine_enemy_kind(10, 0.19), EnemyKind::Bat);
        assert_eq!(determine_enemy_kind(10, 0.21), EnemyKind::Golem);
        assert_eq!(determine_enemy_kind(10, 0.29), EnemyKind::Golem);
        assert_eq!(determine_enemy_kind(10, 0.31), EnemyKind::SlimeII);
        assert_eq!(determine_enemy_kind(10, 0.54), EnemyKind::SlimeII);
        assert_eq!(determine_enemy_kind(10, 0.56), EnemyKind::BatII);
        assert_eq!(determine_enemy_kind(10, 0.79), EnemyKind::BatII);
        assert_eq!(determine_enemy_kind(10, 0.81), EnemyKind::GolemII);
        assert_eq!(determine_enemy_kind(10, 0.99), EnemyKind::GolemII);
        assert_eq!(determine_enemy_kind(50, 0.99), EnemyKind::GolemII);
    }

    #[test]
    fn test_enemy_stats_slime_ii() {
        let config = default_enemy_config();
        // Floor 10: slime base = (40, 15, round(7.0)=7, 70.0)
        // SlimeII: hp*2.0=80, atk*1.5=22, def*1.5=10, speed*0.8=56.0
        let (hp, atk, def, speed) = enemy_stats(EnemyKind::SlimeII, 10, &config);
        assert_eq!(hp, 80);
        assert_eq!(atk, 22);
        assert_eq!(def, 10);
        assert!((speed - 56.0).abs() < 0.1);
    }

    #[test]
    fn test_enemy_stats_bat_ii() {
        let config = default_enemy_config();
        // Floor 10: slime base = (40, 15, round(7.0)=7, 70.0)
        // BatII: hp*0.8=32, atk*1.2=18, def*0.5=3, speed*2.0=140.0
        let (hp, atk, def, speed) = enemy_stats(EnemyKind::BatII, 10, &config);
        assert_eq!(hp, 32);
        assert_eq!(atk, 18);
        assert_eq!(def, 3);
        assert!((speed - 140.0).abs() < 0.1);
    }

    #[test]
    fn test_enemy_stats_golem_ii() {
        let config = default_enemy_config();
        // Floor 10: slime base = (40, 15, round(7.0)=7, 70.0)
        // GolemII: hp*4.0=160, atk*2.0=30, def*2.8=19, speed*0.4=28.0
        let (hp, atk, def, speed) = enemy_stats(EnemyKind::GolemII, 10, &config);
        assert_eq!(hp, 160);
        assert_eq!(atk, 30);
        assert_eq!(def, 19);
        assert!((speed - 28.0).abs() < 0.1);
    }

    #[test]
    fn test_meta_index_in_bounds() {
        let all_kinds = [
            EnemyKind::Slime,
            EnemyKind::Bat,
            EnemyKind::Golem,
            EnemyKind::SlimeII,
            EnemyKind::BatII,
            EnemyKind::GolemII,
        ];
        for kind in all_kinds {
            assert!(
                kind.meta_index() < ENEMY_KIND_META.len(),
                "{kind:?} meta_index {} out of bounds (len={})",
                kind.meta_index(),
                ENEMY_KIND_META.len()
            );
        }
    }

    #[test]
    fn test_enemy_hold_probability() {
        let p1 = enemy_hold_probability(1);
        assert!((p1 - 0.05).abs() < 0.01, "floor 1: {p1}");

        let p25 = enemy_hold_probability(25);
        // 0.05 + (24/49) * 0.75 ≈ 0.05 + 0.3673 ≈ 0.417
        assert!((p25 - 0.42).abs() < 0.02, "floor 25: {p25}");

        let p50 = enemy_hold_probability(50);
        // 0.05 + (49/49) * 0.75 = 0.80
        assert!((p50 - 0.80).abs() < 0.01, "floor 50: {p50}");
    }

    #[test]
    fn test_should_enemy_hold_item() {
        // floor 1: probability ≈ 0.05
        assert!(should_enemy_hold_item(1, 0.0));
        assert!(should_enemy_hold_item(1, 0.04));
        assert!(!should_enemy_hold_item(1, 0.06));
        assert!(!should_enemy_hold_item(1, 0.99));

        // floor 50: probability = 0.80
        assert!(should_enemy_hold_item(50, 0.0));
        assert!(should_enemy_hold_item(50, 0.79));
        assert!(!should_enemy_hold_item(50, 0.81));
    }

    #[test]
    fn test_enemy_equippable_item_kind() {
        use crate::components::item::ItemKind;
        assert_eq!(enemy_equippable_item_kind(0.0), ItemKind::Weapon);
        assert_eq!(enemy_equippable_item_kind(0.19), ItemKind::Weapon);
        assert_eq!(enemy_equippable_item_kind(0.21), ItemKind::Head);
        assert_eq!(enemy_equippable_item_kind(0.39), ItemKind::Head);
        assert_eq!(enemy_equippable_item_kind(0.41), ItemKind::Torso);
        assert_eq!(enemy_equippable_item_kind(0.59), ItemKind::Torso);
        assert_eq!(enemy_equippable_item_kind(0.61), ItemKind::Legs);
        assert_eq!(enemy_equippable_item_kind(0.79), ItemKind::Legs);
        assert_eq!(enemy_equippable_item_kind(0.81), ItemKind::Shield);
        assert_eq!(enemy_equippable_item_kind(0.99), ItemKind::Shield);
    }

    #[test]
    fn test_equip_meta_bounds() {
        let all_kinds = [
            EnemyKind::Slime,
            EnemyKind::Bat,
            EnemyKind::Golem,
            EnemyKind::SlimeII,
            EnemyKind::BatII,
            EnemyKind::GolemII,
        ];
        for kind in all_kinds {
            let meta = &ENEMY_KIND_META[kind.meta_index()];
            assert!(
                meta.equip_slots_min <= meta.equip_slots_max,
                "{kind:?}: min {} > max {}",
                meta.equip_slots_min,
                meta.equip_slots_max
            );
            assert!(
                meta.equip_slots_max <= 3,
                "{kind:?}: max {} > 3",
                meta.equip_slots_max
            );
        }
    }
}
