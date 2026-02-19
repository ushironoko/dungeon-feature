mod camera;
pub mod combat;
mod core;
mod dungeon;
pub mod enemy;
mod hud;
mod inventory;
pub mod item;
mod menu;
mod player;
pub mod transfer;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub struct DungeonFeaturePlugins;

impl PluginGroup for DungeonFeaturePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(core::CorePlugin)
            .add(menu::MenuPlugin)
            .add(dungeon::DungeonPlugin)
            .add(player::PlayerPlugin)
            .add(enemy::EnemyPlugin)
            .add(combat::CombatPlugin)
            .add(item::ItemPlugin)
            .add(hud::HudPlugin)
            .add(camera::CameraPlugin)
            .add(inventory::InventoryPlugin)
            .add(transfer::TransferPlugin)
    }
}
