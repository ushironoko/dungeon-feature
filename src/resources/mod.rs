pub mod bsp;
pub mod dungeon;
pub mod font;
pub mod player_state;
pub mod sprite_assets;
pub mod transfer_state;

pub use dungeon::*;
pub use font::*;
pub use player_state::*;
pub use sprite_assets::*;
pub use transfer_state::*;

use bevy::prelude::*;

use crate::plugins::item::CharmEffects;

#[derive(Resource, Default)]
pub struct ActiveCharmEffects(pub CharmEffects);

#[derive(Resource, Default)]
pub struct RegenTimer(pub f32);
