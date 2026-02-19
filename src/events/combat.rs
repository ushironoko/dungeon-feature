use bevy::prelude::*;

#[derive(Message)]
pub struct DamageEvent {
    pub source: Entity,
    pub target: Entity,
    pub amount: u32,
}
