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

#[derive(Component)]
#[require(FloorEntity)]
pub struct AttackEffect {
    pub remaining: f32,
}
