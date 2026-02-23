use bevy::prelude::*;

use crate::components::{FloatingDamageText, FloorEntity, Knockback, Player};
use crate::config::GameConfig;
use crate::events::{DamageApplied, EnemyDeathMessage};
use crate::resources::{FloorMap, apply_movement_with_collision};
use crate::states::{GameState, PlayingSet};

pub struct FeedbackPlugin;

#[derive(Resource, Default)]
pub struct ScreenShake {
    pub remaining: f32,
    pub intensity: f32,
    pub initial_duration: f32,
}

#[derive(Resource, Default)]
pub struct HitStop {
    pub remaining: f32,
}

#[derive(Component, Clone, Copy)]
pub struct DeathParticle {
    pub lifetime: f32,
    pub velocity: Vec2,
}

impl Plugin for FeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenShake>()
            .init_resource::<HitStop>()
            .add_systems(
                Update,
                (
                    spawn_floating_damage,
                    apply_knockback_on_damage,
                    trigger_screen_shake,
                    trigger_hit_stop,
                    spawn_death_particles,
                )
                    .in_set(PlayingSet::CombatFeedback)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    update_floating_damage,
                    update_knockback,
                    update_screen_shake,
                    update_hit_stop,
                    update_death_particles,
                )
                    .in_set(PlayingSet::PostCombat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_floating_damage(
    mut commands: Commands,
    mut events: MessageReader<DamageApplied>,
    player_query: Query<Entity, With<Player>>,
    config: Res<GameConfig>,
) {
    for event in events.read() {
        let is_player = player_query.contains(event.target);
        let color = if is_player {
            Color::srgb(1.0, 0.3, 0.3)
        } else {
            Color::WHITE
        };

        commands.spawn((
            Text2d::new(format!("{}", event.amount)),
            TextColor(color),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            Transform::from_xyz(event.position.x, event.position.y + 10.0, 10.0),
            FloatingDamageText {
                lifetime: config.combat.damage_text_lifetime,
                velocity: Vec2::new(0.0, config.combat.damage_text_rise_speed),
            },
            FloorEntity,
        ));
    }
}

fn update_floating_damage(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut FloatingDamageText,
        &mut TextColor,
    )>,
    config: Res<GameConfig>,
) {
    for (entity, mut transform, mut text, mut color) in &mut query {
        text.lifetime -= time.delta_secs();
        if text.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation.x += text.velocity.x * time.delta_secs();
        transform.translation.y += text.velocity.y * time.delta_secs();

        let alpha = (text.lifetime / config.combat.damage_text_lifetime).clamp(0.0, 1.0);
        color.0 = color.0.with_alpha(alpha);
    }
}

fn apply_knockback_on_damage(
    mut commands: Commands,
    mut events: MessageReader<DamageApplied>,
    config: Res<GameConfig>,
) {
    for event in events.read() {
        let direction = (event.position - event.source_position).normalize_or_zero();
        if let Ok(mut entity_commands) = commands.get_entity(event.target) {
            entity_commands.insert(Knockback {
                direction,
                remaining: config.combat.knockback_duration,
                speed: config.combat.knockback_speed,
            });
        }
    }
}

fn update_knockback(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Knockback)>,
    floor_map: Res<FloorMap>,
    config: Res<GameConfig>,
) {
    let tile_size = config.dungeon.tile_size;
    for (entity, mut transform, mut knockback) in &mut query {
        knockback.remaining -= time.delta_secs();
        if knockback.remaining <= 0.0 {
            commands.entity(entity).remove::<Knockback>();
            continue;
        }

        let velocity = knockback.direction * knockback.speed * time.delta_secs();
        transform.translation =
            apply_movement_with_collision(transform.translation, velocity, &floor_map, tile_size);
    }
}

fn trigger_screen_shake(
    mut events: MessageReader<DamageApplied>,
    mut shake: ResMut<ScreenShake>,
    player_query: Query<Entity, With<Player>>,
    config: Res<GameConfig>,
) {
    for event in events.read() {
        if player_query.contains(event.target) {
            shake.remaining = config.combat.screen_shake_duration;
            shake.intensity = config.combat.screen_shake_intensity;
            shake.initial_duration = config.combat.screen_shake_duration;
        }
    }
}

fn update_screen_shake(time: Res<Time>, mut shake: ResMut<ScreenShake>) {
    if shake.remaining > 0.0 {
        shake.remaining = (shake.remaining - time.delta_secs()).max(0.0);
    }
}

fn trigger_hit_stop(
    mut events: MessageReader<DamageApplied>,
    mut hit_stop: ResMut<HitStop>,
    mut virtual_time: ResMut<Time<Virtual>>,
    config: Res<GameConfig>,
) {
    for _ in events.read() {
        hit_stop.remaining = config.combat.hit_stop_duration;
        virtual_time.set_relative_speed(0.0);
    }
}

fn update_hit_stop(
    mut hit_stop: ResMut<HitStop>,
    mut virtual_time: ResMut<Time<Virtual>>,
    real_time: Res<Time<Real>>,
) {
    if hit_stop.remaining <= 0.0 {
        return;
    }
    hit_stop.remaining -= real_time.delta_secs();
    if hit_stop.remaining <= 0.0 {
        virtual_time.set_relative_speed(1.0);
    }
}

fn spawn_death_particles(mut commands: Commands, mut events: MessageReader<EnemyDeathMessage>) {
    for event in events.read() {
        let particle_count = 5;
        for i in 0..particle_count {
            let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
            let velocity = Vec2::new(angle.cos(), angle.sin()) * 80.0;

            commands.spawn((
                Sprite::from_color(Color::srgb(0.9, 0.6, 0.2), Vec2::splat(4.0)),
                Transform::from_xyz(event.position.x, event.position.y, 5.0),
                DeathParticle {
                    lifetime: 0.4,
                    velocity,
                },
                FloorEntity,
            ));
        }
    }
}

fn update_death_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut DeathParticle, &mut Sprite)>,
) {
    for (entity, mut transform, mut particle, mut sprite) in &mut query {
        particle.lifetime -= time.delta_secs();
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation.x += particle.velocity.x * time.delta_secs();
        transform.translation.y += particle.velocity.y * time.delta_secs();

        let alpha = (particle.lifetime / 0.4).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(alpha);
    }
}
