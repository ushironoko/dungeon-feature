use bevy::prelude::*;

use crate::config::GameConfig;
use crate::events::{DamageEvent, EnemyDeathMessage};
use crate::resources::player_state::PlayerState;
use crate::resources::transfer_state::TransferState;
use crate::resources::{CurrentFloor, DungeonRng};
use crate::states::{FloorTransitionSetup, GameState, PlayingSet};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        let config = GameConfig::default();
        let player_state = PlayerState {
            current_hp: config.player.hp,
            equipment: Default::default(),
            inventory: Default::default(),
        };
        let transfer_state = TransferState::new(config.transfer.charges_per_run);
        app.init_state::<GameState>()
            .insert_resource(config)
            .init_resource::<CurrentFloor>()
            .init_resource::<DungeonRng>()
            .insert_resource(player_state)
            .insert_resource(transfer_state)
            .add_message::<DamageEvent>()
            .add_message::<EnemyDeathMessage>()
            .configure_sets(
                OnEnter(GameState::FloorTransition),
                (
                    FloorTransitionSetup::Cleanup,
                    FloorTransitionSetup::AdvanceFloor,
                    FloorTransitionSetup::GenerateFloor,
                    FloorTransitionSetup::RenderTiles,
                    FloorTransitionSetup::SpawnEntities,
                    FloorTransitionSetup::Complete,
                )
                    .chain(),
            )
            .configure_sets(
                Update,
                (
                    PlayingSet::Player,
                    PlayingSet::Enemy,
                    PlayingSet::Combat,
                    PlayingSet::Item,
                    PlayingSet::PostCombat,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Loading), skip_to_menu)
            .add_systems(
                OnEnter(GameState::FloorTransition),
                return_to_playing.in_set(FloorTransitionSetup::Complete),
            );
    }
}

fn skip_to_menu(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Menu);
}

fn return_to_playing(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}
