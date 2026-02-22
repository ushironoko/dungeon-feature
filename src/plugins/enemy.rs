use bevy::prelude::*;
use rand::Rng;

use crate::components::{
    AiState, Attack, AttackCooldown, AttackRange, ChaseLostTimer, Dead, Defense, DetectionRadius,
    Enemy, FloorEntity, Health, Player, Speed, WanderDirection, WanderInterval, WanderTimer,
};
use crate::config::GameConfig;
use crate::events::DamageEvent;
use crate::plugins::combat::{
    ENEMY_KIND_META, calculate_damage, determine_enemy_kind, enemy_stats,
};
use crate::resources::sprite_assets::{SpriteAssets, make_sprite};
use crate::resources::{
    ActiveCharmEffects, CurrentFloor, DungeonRng, FloorMap, apply_movement_with_collision,
    pixel_to_tile,
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

/// プレイヤーからの最低スポーン距離（タイル数）
const MIN_SPAWN_DISTANCE_TILES: f32 = 8.0;
/// 部屋内でのリトライ回数上限
const SPAWN_POSITION_RETRIES: u32 = 5;

fn spawn_enemies(
    mut commands: Commands,
    floor_map: Res<FloorMap>,
    current_floor: Res<CurrentFloor>,
    config: Res<GameConfig>,
    mut dungeon_rng: ResMut<DungeonRng>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
) {
    let floor = current_floor.number();
    let roll: f32 = dungeon_rng.0.random();
    let count = crate::plugins::combat::enemy_count_random(
        config.enemy.min_count,
        config.enemy.max_count,
        roll,
    );

    let (sx, sy) = floor_map.spawn_point;
    let tile_size = config.dungeon.tile_size;
    let player_pos = Vec2::new(sx as f32 * tile_size, sy as f32 * tile_size);

    spawn_enemy_batch(
        &mut commands,
        &floor_map,
        &config,
        &mut dungeon_rng,
        floor,
        count,
        player_pos,
        &sprite_assets,
        &images,
    );

    // リスポーンタイマー初期化
    commands.insert_resource(RespawnTimer {
        remaining: config.enemy.respawn_interval,
    });

    info!("Spawned {} enemies on floor {}", count, floor);
}

#[allow(clippy::too_many_arguments)]
fn spawn_enemy_batch(
    commands: &mut Commands,
    floor_map: &FloorMap,
    config: &GameConfig,
    dungeon_rng: &mut ResMut<DungeonRng>,
    floor: u32,
    count: u32,
    player_pos: Vec2,
    sprite_assets: &SpriteAssets,
    images: &Assets<Image>,
) {
    let tile_size = config.dungeon.tile_size;
    let min_dist = MIN_SPAWN_DISTANCE_TILES * tile_size;

    let available_rooms = if floor_map.rooms.len() > 1 {
        &floor_map.rooms[1..]
    } else {
        &floor_map.rooms[..]
    };

    if available_rooms.is_empty() {
        return;
    }

    // 部屋をプレイヤーからの距離が遠い順にソート（インデックス配列）
    let mut room_indices: Vec<usize> = (0..available_rooms.len()).collect();
    room_indices.sort_by(|&a, &b| {
        let (ca_x, ca_y) = available_rooms[a].center();
        let (cb_x, cb_y) = available_rooms[b].center();
        let dist_a = Vec2::new(ca_x as f32 * tile_size, ca_y as f32 * tile_size)
            .distance_squared(player_pos);
        let dist_b = Vec2::new(cb_x as f32 * tile_size, cb_y as f32 * tile_size)
            .distance_squared(player_pos);
        dist_b
            .partial_cmp(&dist_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for i in 0..count {
        let kind_roll: f32 = dungeon_rng.0.random();
        let kind = determine_enemy_kind(floor, kind_roll);
        let meta = &ENEMY_KIND_META[kind.meta_index()];
        let (hp, atk, def, speed) = enemy_stats(kind, floor, &config.enemy);

        // 遠い部屋から優先的に使用
        let room_idx = room_indices[i as usize % room_indices.len()];
        let room = &available_rooms[room_idx];

        // 部屋内でプレイヤーから十分離れた位置を探す
        let mut spawn_x = 0;
        let mut spawn_y = 0;
        let mut found_good_pos = false;

        for _ in 0..SPAWN_POSITION_RETRIES {
            let rx = dungeon_rng
                .0
                .random_range(room.x + 1..room.x + room.width - 1);
            let ry = dungeon_rng
                .0
                .random_range(room.y + 1..room.y + room.height - 1);
            let candidate = Vec2::new(rx as f32 * tile_size, ry as f32 * tile_size);
            if candidate.distance(player_pos) >= min_dist {
                spawn_x = rx;
                spawn_y = ry;
                found_good_pos = true;
                break;
            }
            spawn_x = rx;
            spawn_y = ry;
        }

        // リトライでも良い位置が見つからなかった場合、他の遠い部屋を試す
        if !found_good_pos {
            for &alt_idx in &room_indices {
                let alt_room = &available_rooms[alt_idx];
                let (cx, cy) = alt_room.center();
                let center_pos = Vec2::new(cx as f32 * tile_size, cy as f32 * tile_size);
                if center_pos.distance(player_pos) >= min_dist {
                    let rx = dungeon_rng
                        .0
                        .random_range(alt_room.x + 1..alt_room.x + alt_room.width - 1);
                    let ry = dungeon_rng
                        .0
                        .random_range(alt_room.y + 1..alt_room.y + alt_room.height - 1);
                    spawn_x = rx;
                    spawn_y = ry;
                    break;
                }
            }
        }

        let (cr, cg, cb) = meta.color;

        commands.spawn((
            make_sprite(
                sprite_assets.enemy_handle(kind),
                images,
                Color::srgb(cr, cg, cb),
                Color::WHITE,
                Vec2::splat(tile_size * 0.7),
            ),
            Transform::from_xyz(spawn_x as f32 * tile_size, spawn_y as f32 * tile_size, 1.0),
            Enemy,
            kind,
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
            (
                WanderInterval {
                    min: meta.wander_min,
                    max: meta.wander_max,
                },
                DetectionRadius(config.enemy.slime_detection_radius * meta.detection_mult),
                AttackRange(config.enemy.slime_attack_range * meta.attack_range_mult),
                AttackCooldown {
                    remaining: 0.0,
                    duration: config.enemy.slime_attack_cooldown * meta.attack_cooldown,
                },
                ChaseLostTimer(0.0),
            ),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn respawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<RespawnTimer>,
    enemy_query: Query<(), (With<Enemy>, Without<Dead>)>,
    player_query: Query<&Transform, With<Player>>,
    floor_map: Res<FloorMap>,
    current_floor: Res<CurrentFloor>,
    config: Res<GameConfig>,
    mut dungeon_rng: ResMut<DungeonRng>,
    sprite_assets: Res<SpriteAssets>,
    images: Res<Assets<Image>>,
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

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    let spawn_count = config.enemy.min_count - current_count;
    let floor = current_floor.number();

    spawn_enemy_batch(
        &mut commands,
        &floor_map,
        &config,
        &mut dungeon_rng,
        floor,
        spawn_count,
        player_pos,
        &sprite_assets,
        &images,
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
    effects: Res<ActiveCharmEffects>,
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
            &WanderInterval,
        ),
        (With<Enemy>, Without<Dead>),
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
        wander_interval,
    ) in &mut enemy_query
    {
        let enemy_pos = transform.translation.truncate();
        let distance = enemy_pos.distance(player_pos);
        let detection_px = detection_radius.0 * tile_size * (1.0 - effects.0.detection_reduction);

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
                        wander_timer.0 = dungeon_rng
                            .0
                            .random_range(wander_interval.min..wander_interval.max);
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
                        wander_timer.0 = dungeon_rng
                            .0
                            .random_range(wander_interval.min..wander_interval.max);
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
                        wander_timer.0 = dungeon_rng
                            .0
                            .random_range(wander_interval.min..wander_interval.max);
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
        (With<Enemy>, Without<Player>, Without<Dead>),
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

#[allow(clippy::type_complexity)]
fn enemy_attack(
    player_query: Query<(Entity, &Transform, &Defense), With<Player>>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &Attack,
            &mut AttackCooldown,
            &AiState,
            &AttackRange,
        ),
        (With<Enemy>, Without<Dead>),
    >,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok((player_entity, player_transform, player_defense)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    for (enemy_entity, transform, attack, mut cooldown, ai_state, attack_range) in &mut enemy_query
    {
        if *ai_state != AiState::Attack {
            continue;
        }

        let enemy_pos = transform.translation.truncate();
        let distance = enemy_pos.distance(player_pos);

        if distance <= attack_range.0 && cooldown.remaining <= 0.0 {
            let damage = calculate_damage(attack.0, player_defense.0);
            damage_events.write(DamageEvent {
                source: enemy_entity,
                target: player_entity,
                amount: damage,
            });
            cooldown.remaining = cooldown.duration;
        }
    }
}
