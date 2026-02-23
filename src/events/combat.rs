use bevy::prelude::*;

#[derive(Message)]
pub struct DamageEvent {
    pub source: Entity,
    pub target: Entity,
    pub amount: u32,
}

#[derive(Message)]
pub struct DamageApplied {
    pub target: Entity,
    pub amount: u32,
    pub position: Vec2,
    pub source_position: Vec2,
}
