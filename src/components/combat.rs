use bevy::prelude::*;

use super::FloorEntity;

#[derive(Component, Clone, Copy)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[derive(Component, Clone, Copy)]
pub struct Attack(pub u32);

#[derive(Component, Clone, Copy)]
pub struct Defense(pub u32);

#[derive(Component, Clone, Copy)]
pub struct AttackCooldown {
    pub remaining: f32,
    pub duration: f32,
}

#[derive(Component, Clone, Copy)]
pub struct InvincibilityTimer {
    pub remaining: f32,
}

#[derive(Component, Clone, Copy)]
#[require(FloorEntity)]
pub struct AttackEffect {
    pub remaining: f32,
    pub duration: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub initial_alpha: f32,
}

#[derive(Component)]
pub struct Dead;

#[derive(Component, Clone, Copy)]
pub struct FloatingDamageText {
    pub lifetime: f32,
    pub velocity: Vec2,
}

#[derive(Component, Clone, Copy)]
pub struct Knockback {
    pub direction: Vec2,
    pub remaining: f32,
    pub speed: f32,
}
