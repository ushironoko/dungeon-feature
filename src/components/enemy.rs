use bevy::prelude::*;

use super::FloorEntity;

#[derive(Component)]
#[require(FloorEntity)]
pub struct Enemy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiState {
    #[default]
    Idle,
    Wander,
    Chase,
    Attack,
}

#[derive(Component, Clone, Copy)]
pub struct WanderTimer(pub f32);

#[derive(Component, Clone, Copy)]
pub struct WanderDirection(pub Vec2);

#[derive(Component, Clone, Copy)]
pub struct DetectionRadius(pub f32);

#[derive(Component, Clone, Copy)]
pub struct AttackRange(pub f32);

#[derive(Component, Clone, Copy)]
pub struct ChaseLostTimer(pub f32);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnemyKind {
    #[default]
    Slime,
    Bat,
    Golem,
}

#[derive(Component, Clone, Copy)]
pub struct WanderInterval {
    pub min: f32,
    pub max: f32,
}
