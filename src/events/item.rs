use bevy::prelude::*;

#[derive(Message)]
pub struct EnemyDeathMessage {
    pub position: Vec2,
    pub floor: u32,
}
