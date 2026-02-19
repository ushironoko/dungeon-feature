use bevy::prelude::*;
use rand::Rng;

use crate::components::{
    AiState, Attack, AttackCooldown, AttackRange, ChaseLostTimer, Defense, DetectionRadius, Enemy,
    FloorEntity, Health, Player, Speed, WanderDirection, WanderTimer,
};
use crate::config::GameConfig;
use crate::events::DamageEvent;
use crate::plugins::combat::{calculate_damage, enemy_count_random, slime_stats};
use crate::resources::{
    apply_movement_with_collision, pixel_to_tile, CurrentFloor, DungeonRng, FloorMap,
};
use crate::states::{FloorTransitionSetup, GameState, PlayingSet};

pub struct EnemyPlugin;

#[derive(Resource)]
pub struct RespawnTimer {
    pub remaining: f32,
}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::FloorTransition),
            spawn_enemies.in_set(FloorTransitionSetup::SpawnEntities),
        )
        .add_systems(
            Update,
            (enemy_ai, enemy_movement, enemy_attack, respawn_enemies)
                .chain()
                .in_set(PlayingSet::Enemy)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn spawn_enemies(
    mut commands: Commands,
    floor_map: Res<FloorMap>,
    current_floor: Res<CurrentFloor>,
    config: Res<GameConfig>,
    mut dungeon_rng: ResMut<DungeonRng>,
) {
    let floor = current_floor.number();
    let roll: f32 = dungeon_rng.0.random();
    let count = enemy_count_random(config.enemy.min_count, config.enemy.max_count, roll);

    spawn_enemy_batch(&mut commands, &floor_map, &config, &mut dungeon_rng, floor, count);

    // リスポーンタイマー初期化
    commands.insert_resource(RespawnTimer {
        remaining: config.enemy.respawn_interval,
    });

    info!("Spawned {} enemies on floor {}", count, floor);
}

fn spawn_enemy_batch(
    commands: &mut Commands,
    floor_map: &FloorMap,
    config: &GameConfig,
    dungeon_rng: &mut ResMut<DungeonRng>,
    floor: u32,
    count: u32,
) {
    let (hp, atk, def, speed) = slime_stats(floor, &config.enemy);
    let tile_size = config.dungeon.tile_size;

    let available_rooms = if floor_map.rooms.len() > 1 {
        &floor_map.rooms[1..]
    } else {
        &floor_map.rooms[..]
    };

    if available_rooms.is_empty() {
        return;
    }

    for i in 0..count {
        let room = &available_rooms[i as usize % available_rooms.len()];

        let rx = dungeon_rng.0.random_range(room.x + 1..room.x + room.width - 1);
        let ry = dungeon_rng.0.random_range(room.y + 1..room.y + room.height - 1);

        let (spawn_x, spawn_y) = if floor_map.rooms.len() == 1 {
            let (sx, sy) = floor_map.spawn_point;
            let corners = [
                (room.x + 1, room.y + 1),
                (room.x + room.width - 2, room.y + 1),
                (room.x + 1, room.y + room.height - 2),
                (room.x + room.width - 2, room.y + room.height - 2),
            ];
            let fallback = (rx, ry);
            let best = corners
                .iter()
                .max_by_key(|(cx, cy)| {
                    let dx = *cx - sx;
                    let dy = *cy - sy;
                    dx * dx + dy * dy
                })
                .unwrap_or(&fallback);
            let offset_x = dungeon_rng.0.random_range(-1..=1);
            let offset_y = dungeon_rng.0.random_range(-1..=1);
            let fx = (best.0 + offset_x).clamp(room.x + 1, room.x + room.width - 2);
            let fy = (best.1 + offset_y).clamp(room.y + 1, room.y + room.height - 2);
            (fx, fy)
        } else {
            (rx, ry)
        };

        commands.spawn((
            Sprite::from_color(
                Color::srgb(0.2, 0.8, 0.3),
                Vec2::splat(tile_size * 0.7),
            ),
            Transform::from_xyz(
                spawn_x as f32 * tile_size,
                spawn_y as f32 * tile_size,
                1.0,
            ),
            Enemy,
            FloorEntity,
            Health {
                current: hp,
                max: hp,
            },
            Attack(atk),
            Defense(def),
            Speed(speed),
            AiState::Idle,
            WanderTimer(0.0),
            WanderDirection(Vec2::ZERO),
            DetectionRadius(config.enemy.slime_detection_radius),
            AttackRange(config.enemy.slime_attack_range),
            AttackCooldown {
                remaining: 0.0,
                duration: config.enemy.slime_attack_cooldown,
            },
            ChaseLostTimer(0.0),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn respawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<RespawnTimer>,
    enemy_query: Query<(), With<Enemy>>,
    floor_map: Res<FloorMap>,
    current_floor: Res<CurrentFloor>,
    config: Res<GameConfig>,
    mut dungeon_rng: ResMut<DungeonRng>,
) {
    timer.remaining -= time.delta_secs();
    if timer.remaining > 0.0 {
        return;
    }
    timer.remaining = config.enemy.respawn_interval;

    let current_count = enemy_query.iter().count() as u32;
    if current_count >= config.enemy.min_count {
        return;
    }

    let spawn_count = config.enemy.min_count - current_count;
    let floor = current_floor.number();

    spawn_enemy_batch(
        &mut commands,
        &floor_map,
        &config,
        &mut dungeon_rng,
        floor,
        spawn_count,
    );

    info!(
        "Respawned {} enemies (total: {})",
        spawn_count,
        current_count + spawn_count
    );
}

#[allow(clippy::type_complexity)]
fn enemy_ai(
    time: Res<Time>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (
            &Transform,
            &mut AiState,
            &DetectionRadius,
            &AttackRange,
            &AttackCooldown,
            &mut WanderTimer,
            &mut WanderDirection,
            &mut ChaseLostTimer,
        ),
        With<Enemy>,
    >,
    mut dungeon_rng: ResMut<DungeonRng>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();
    let tile_size = config.dungeon.tile_size;

    for (
        transform,
        mut ai_state,
        detection_radius,
        attack_range,
        cooldown,
        mut wander_timer,
        mut wander_dir,
        mut chase_lost,
    ) in &mut enemy_query
    {
        let enemy_pos = transform.translation.truncate();
        let distance = enemy_pos.distance(player_pos);
        let detection_px = detection_radius.0 * tile_size;

        let enemy_tile = pixel_to_tile(enemy_pos, tile_size);
        let player_tile = pixel_to_tile(player_pos, tile_size);
        let has_los = floor_map.has_line_of_sight_tiles(enemy_tile, player_tile);

        let can_see_player = distance <= detection_px && has_los;

        match *ai_state {
            AiState::Idle => {
                if can_see_player {
                    *ai_state = AiState::Chase;
                    chase_lost.0 = 0.0;
                } else {
                    wander_timer.0 -= time.delta_secs();
                    if wander_timer.0 <= 0.0 {
                        *ai_state = AiState::Wander;
                        wander_timer.0 = dungeon_rng.0.random_range(2.0..4.0);
                        let angle: f32 = dungeon_rng.0.random_range(0.0..std::f32::consts::TAU);
                        wander_dir.0 = Vec2::new(angle.cos(), angle.sin());
                    }
                }
            }
            AiState::Wander => {
                if can_see_player {
                    *ai_state = AiState::Chase;
                    chase_lost.0 = 0.0;
                } else {
                    wander_timer.0 -= time.delta_secs();
                    if wander_timer.0 <= 0.0 {
                        *ai_state = AiState::Idle;
                        wander_timer.0 = dungeon_rng.0.random_range(2.0..4.0);
                    }
                }
            }
            AiState::Chase => {
                if !can_see_player {
                    chase_lost.0 += time.delta_secs();
                    if chase_lost.0 >= config.enemy.slime_chase_lost_time
                        || distance > detection_px * 1.2
                    {
                        *ai_state = AiState::Idle;
                        wander_timer.0 = dungeon_rng.0.random_range(2.0..4.0);
                        chase_lost.0 = 0.0;
                    }
                } else {
                    chase_lost.0 = 0.0;
                    if distance <= attack_range.0 && cooldown.remaining <= 0.0 {
                        *ai_state = AiState::Attack;
                    }
                }
            }
            AiState::Attack => {
                *ai_state = AiState::Chase;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn enemy_movement(
    time: Res<Time>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (&mut Transform, &Speed, &AiState, &WanderDirection),
        (With<Enemy>, Without<Player>),
    >,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();
    let tile_size = config.dungeon.tile_size;

    for (mut transform, speed, ai_state, wander_dir) in &mut enemy_query {
        let velocity = match ai_state {
            AiState::Chase | AiState::Attack => {
                let enemy_pos = transform.translation.truncate();
                let direction = (player_pos - enemy_pos).normalize_or_zero();
                direction * speed.0 * time.delta_secs()
            }
            AiState::Wander => wander_dir.0 * speed.0 * 0.5 * time.delta_secs(),
            AiState::Idle => Vec2::ZERO,
        };

        if velocity != Vec2::ZERO {
            transform.translation = apply_movement_with_collision(
                transform.translation,
                velocity,
                &floor_map,
                tile_size,
            );
        }
    }
}

fn enemy_attack(
    player_query: Query<(Entity, &Transform, &Defense), With<Player>>,
    mut enemy_query: Query<
        (
            &Transform,
            &Attack,
            &mut AttackCooldown,
            &AiState,
            &AttackRange,
        ),
        With<Enemy>,
    >,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok((player_entity, player_transform, player_defense)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    for (transform, attack, mut cooldown, ai_state, attack_range) in &mut enemy_query {
        if *ai_state != AiState::Attack {
            continue;
        }

        let enemy_pos = transform.translation.truncate();
        let distance = enemy_pos.distance(player_pos);

        if distance <= attack_range.0 && cooldown.remaining <= 0.0 {
            let damage = calculate_damage(attack.0, player_defense.0);
            damage_events.write(DamageEvent {
                source: Entity::PLACEHOLDER,
                target: player_entity,
                amount: damage,
            });
            cooldown.remaining = cooldown.duration;
        }
    }
}
