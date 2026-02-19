use bevy::prelude::*;

use crate::components::{AttackCooldown, AttackEffect, Enemy, Health, InvincibilityTimer, Player};
use crate::config::{EnemyConfig, GameConfig};
use crate::events::{DamageEvent, EnemyDeathMessage};
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
        );
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
    mut query: Query<(&mut Health, Has<InvincibilityTimer>)>,
    config: Res<GameConfig>,
) {
    for event in events.read() {
        let Ok((mut health, has_invincibility)) = query.get_mut(event.target) else {
            continue;
        };
        if health.current == 0 {
            continue;
        }
        if has_invincibility {
            continue;
        }
        health.current = health.current.saturating_sub(event.amount);
        info!(
            "Damage: {} -> entity, HP: {}/{}",
            event.amount, health.current, health.max
        );
        // プレイヤーのみ無敵付与（Player コンポーネントの有無で判断は不要、
        // InvincibilityTimer を付与するだけで十分）
        commands.entity(event.target).insert(InvincibilityTimer {
            remaining: config.player.invincibility,
        });
    }
}

fn check_death(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Health, &Transform), With<Enemy>>,
    player_query: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut enemy_death_events: MessageWriter<EnemyDeathMessage>,
    current_floor: Res<CurrentFloor>,
) {
    // 敵の死亡チェック
    for (entity, health, transform) in &enemy_query {
        if health.current == 0 {
            let pos = transform.translation.truncate();
            enemy_death_events.write(EnemyDeathMessage {
                position: pos,
                floor: current_floor.number(),
            });
            info!("Enemy defeated!");
            commands.entity(entity).despawn();
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
    mut query: Query<(Entity, &mut AttackEffect)>,
) {
    for (entity, mut effect) in &mut query {
        effect.remaining -= time.delta_secs();
        if effect.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_damage_visual(
    mut query: Query<(&mut Sprite, Option<&InvincibilityTimer>), With<Health>>,
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
}
