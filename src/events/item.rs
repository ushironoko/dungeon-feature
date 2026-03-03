use bevy::prelude::*;

use crate::components::item::ItemSpec;

#[derive(Message)]
pub struct EnemyDeathMessage {
    pub position: Vec2,
    pub floor: u32,
}

#[derive(Message)]
pub struct EnemyEquipmentDropMessage {
    pub position: Vec2,
    pub floor: u32,
    pub items: [Option<ItemSpec>; 3],
}
