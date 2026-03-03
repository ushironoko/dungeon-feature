use bevy::prelude::*;

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use bevy::sprite::Anchor;

use crate::components::{
    Attack, AttackCooldown, AttackEffect, CameraFollow, DashActive, Defense, Enemy,
    FacingDirection, Health, Player, Speed, TileKind,
};
use crate::config::GameConfig;
use crate::events::DamageEvent;
use crate::plugins::combat::{calculate_damage, is_in_attack_fan};
use crate::resources::player_state::PlayerState;
use crate::resources::sprite_assets::{SpriteAssets, make_sprite};
use crate::resources::{ActiveCharmEffects, FloorMap, RegenTimer, apply_movement_with_collision};
use crate::states::{FloorTransitionSetup, GameState, PlayingSet};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DashActive>()
            .add_systems(
                OnEnter(GameState::FloorTransition),
                spawn_player.in_set(FloorTransitionSetup::SpawnEntities),
            )
            .add_systems(
                Update,
                (toggle_dash, player_movement, player_attack)
                    .chain()
                    .in_set(PlayingSet::Player)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    sync_player_state,
                    charm_regen,
                    check_stairs,
                    check_treasure_chest,
                )
                    .chain()
                    .in_set(PlayingSet::PostCombat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_player(
    mut commands: Commands,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    player_state: Res<PlayerState>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
) {
    let (sx, sy) = floor_map.spawn_point;
    let tile_size = config.dungeon.tile_size;

    commands.spawn((
        make_sprite(
            &sprite_assets.player,
            &images,
            Color::srgb(0.2, 0.4, 0.9),
            Color::WHITE,
            Vec2::splat(tile_size * 0.8),
        ),
        Transform::from_xyz(sx as f32 * tile_size, sy as f32 * tile_size, 1.0),
        Player,
        Speed(config.player.speed),
        FacingDirection(Vec2::new(0.0, -1.0)),
        CameraFollow,
        Health {
            current: player_state.current_hp,
            max: config.player.hp,
        },
        Attack(config.player.attack),
        Defense(config.player.defense),
        AttackCooldown {
            remaining: 0.0,
            duration: config.player.attack_cooldown,
        },
    ));
}

fn toggle_dash(keyboard: Res<ButtonInput<KeyCode>>, mut dash: ResMut<DashActive>) {
    if keyboard.just_pressed(KeyCode::ShiftLeft) || keyboard.just_pressed(KeyCode::ShiftRight) {
        dash.0 = !dash.0;
    }
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Speed, &mut FacingDirection), With<Player>>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    dash: Res<DashActive>,
) {
    let Ok((mut transform, speed, mut facing)) = query.single_mut() else {
        return;
    };

    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction == Vec2::ZERO {
        return;
    }

    direction = direction.normalize();
    facing.0 = direction;

    let tile_size = config.dungeon.tile_size;
    let multiplier = if dash.0 {
        config.player.dash_multiplier
    } else {
        1.0
    };
    let velocity = direction * speed.0 * multiplier * time.delta_secs();

    transform.translation =
        apply_movement_with_collision(transform.translation, velocity, &floor_map, tile_size);
}

#[allow(clippy::too_many_arguments)]
fn player_attack(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<
        (
            Entity,
            &Transform,
            &FacingDirection,
            &Attack,
            &mut AttackCooldown,
        ),
        With<Player>,
    >,
    enemy_query: Query<(Entity, &Transform, &Defense), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
    config: Res<GameConfig>,
    effects: Res<ActiveCharmEffects>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok((player_entity, transform, facing, attack, mut cooldown)) = player_query.single_mut()
    else {
        return;
    };

    if cooldown.remaining > 0.0 {
        return;
    }

    cooldown.remaining = cooldown.duration * (1.0 - effects.0.cooldown_reduction);

    let player_pos = transform.translation.truncate();
    let attack_range = config.player.attack_range;
    let attack_angle = config.player.attack_angle;

    // 剣スラッシュエフェクトをスポーン
    let attack_duration = config.combat.attack_effect_duration;
    let base_angle = facing.0.y.atan2(facing.0.x) - FRAC_PI_2;
    let start_angle = base_angle + FRAC_PI_4;
    let end_angle = base_angle - FRAC_PI_4;

    let sword_size = Vec2::new(attack_range * 0.3, attack_range * 0.8);
    let sword_sprite = make_sprite(
        &sprite_assets.attack_sword,
        &images,
        Color::srgba(1.0, 1.0, 0.5, 0.6),
        Color::WHITE,
        sword_size,
    );
    let initial_alpha = sword_sprite.color.alpha();

    commands.spawn((
        sword_sprite,
        Anchor::BOTTOM_CENTER,
        Transform {
            translation: Vec3::new(player_pos.x, player_pos.y, 2.0),
            rotation: Quat::from_rotation_z(start_angle),
            ..default()
        },
        AttackEffect {
            remaining: attack_duration,
            duration: attack_duration,
            start_angle,
            end_angle,
            initial_alpha,
        },
    ));

    // 扇形範囲判定
    for (enemy_entity, enemy_transform, enemy_defense) in &enemy_query {
        let enemy_pos = enemy_transform.translation.truncate();
        if is_in_attack_fan(player_pos, facing.0, enemy_pos, attack_range, attack_angle) {
            let damage = calculate_damage(attack.0, enemy_defense.0);
            damage_events.write(DamageEvent {
                source: player_entity,
                target: enemy_entity,
                amount: damage,
            });
        }
    }
}

fn sync_player_state(query: Query<&Health, With<Player>>, mut player_state: ResMut<PlayerState>) {
    let Ok(health) = query.single() else {
        return;
    };
    if player_state.current_hp != health.current {
        player_state.current_hp = health.current;
    }
}

fn check_stairs(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Query<&Transform, With<Player>>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    // 最終階では stairs_position=None なのでスキップされる
    if floor_map.stairs_position.is_none() {
        return;
    }

    let Ok(transform) = query.single() else {
        return;
    };

    let tile_size = config.dungeon.tile_size;
    let tile_x = (transform.translation.x / tile_size).round() as i32;
    let tile_y = (transform.translation.y / tile_size).round() as i32;

    if floor_map.tile_at(tile_x, tile_y) == Some(TileKind::Stairs) {
        info!("Descending stairs...");
        next_state.set(GameState::FloorTransition);
    }
}

fn charm_regen(
    time: Res<Time>,
    effects: Res<ActiveCharmEffects>,
    mut regen_timer: ResMut<RegenTimer>,
    mut query: Query<&mut Health, With<Player>>,
) {
    if effects.0.regen_amount == 0 || effects.0.regen_interval <= 0.0 {
        return;
    }
    regen_timer.0 += time.delta_secs();
    if regen_timer.0 >= effects.0.regen_interval {
        regen_timer.0 -= effects.0.regen_interval;
        let Ok(mut health) = query.single_mut() else {
            return;
        };
        if health.current < health.max {
            health.current = (health.current + effects.0.regen_amount).min(health.max);
            info!("Charm regen: HP {}/{}", health.current, health.max);
        }
    }
}

fn check_treasure_chest(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Query<&Transform, With<Player>>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    if floor_map.treasure_chest_position.is_none() {
        return;
    }

    let Ok(transform) = query.single() else {
        return;
    };

    let tile_size = config.dungeon.tile_size;
    let tile_x = (transform.translation.x / tile_size).round() as i32;
    let tile_y = (transform.translation.y / tile_size).round() as i32;

    if floor_map.tile_at(tile_x, tile_y) == Some(TileKind::TreasureChest) {
        info!("Opening treasure chest... Ending!");
        next_state.set(GameState::Ending);
    }
}
