use bevy::prelude::*;

use crate::config::GameConfig;
use crate::events::{DamageApplied, DamageEvent, EnemyDeathMessage, EnemyEquipmentDropMessage};
use crate::resources::font::GameFont;
use crate::resources::player_state::PlayerState;
use crate::resources::sprite_assets::SpriteAssets;
use crate::resources::transfer_state::{TransferArrivalNotice, TransferState};
use crate::resources::{ActiveCharmEffects, CurrentFloor, DungeonRng, RegenTimer};
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
        app.add_systems(Startup, (load_game_font, load_sprite_assets))
            .init_state::<GameState>()
            .insert_resource(config)
            .init_resource::<CurrentFloor>()
            .init_resource::<DungeonRng>()
            .insert_resource(player_state)
            .insert_resource(transfer_state)
            .add_message::<DamageEvent>()
            .add_message::<DamageApplied>()
            .add_message::<EnemyDeathMessage>()
            .add_message::<EnemyEquipmentDropMessage>()
            .init_resource::<TransferArrivalNotice>()
            .init_resource::<ActiveCharmEffects>()
            .init_resource::<RegenTimer>()
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
                    PlayingSet::CombatFeedback,
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

fn load_game_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/NotoSansJP-Regular.otf");
    commands.insert_resource(GameFont(font));
}

fn load_sprite_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SpriteAssets {
        player: asset_server.load("sprites/player.png"),
        slime: asset_server.load("sprites/enemies/slime.png"),
        bat: asset_server.load("sprites/enemies/bat.png"),
        golem: asset_server.load("sprites/enemies/golem.png"),
        slime_ii: asset_server.load("sprites/enemies/slime_ii.png"),
        bat_ii: asset_server.load("sprites/enemies/bat_ii.png"),
        golem_ii: asset_server.load("sprites/enemies/golem_ii.png"),
        item_weapon: asset_server.load("sprites/items/weapon.png"),
        item_head: asset_server.load("sprites/items/head.png"),
        item_torso: asset_server.load("sprites/items/torso.png"),
        item_legs: asset_server.load("sprites/items/legs.png"),
        item_shield: asset_server.load("sprites/items/shield.png"),
        item_charm: asset_server.load("sprites/items/charm.png"),
        item_backpack: asset_server.load("sprites/items/backpack.png"),
        item_potion: asset_server.load("sprites/items/potion.png"),
        attack_sword: asset_server.load("sprites/attack_sword.png"),
    });
}

fn skip_to_menu(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Menu);
}

fn return_to_playing(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}
