use bevy::prelude::*;

use crate::components::{FloorEntity, Stairs, Tile, TileKind, TilePosition, TreasureChest};
use crate::config::GameConfig;
use crate::resources::bsp::generate_bsp_floor;
use crate::resources::{CurrentFloor, DungeonRng, FloorMap};
use crate::states::{FloorTransitionSetup, GameState};

pub struct DungeonPlugin;

impl Plugin for DungeonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::FloorTransition),
            (
                cleanup_floor.in_set(FloorTransitionSetup::Cleanup),
                advance_floor.in_set(FloorTransitionSetup::AdvanceFloor),
                generate_floor.in_set(FloorTransitionSetup::GenerateFloor),
                render_tiles.in_set(FloorTransitionSetup::RenderTiles),
            ),
        );
    }
}

fn cleanup_floor(mut commands: Commands, query: Query<Entity, With<FloorEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn advance_floor(mut current_floor: ResMut<CurrentFloor>) {
    current_floor.advance();
    info!("Advancing to floor {}", current_floor.number());
}

fn generate_floor(
    mut commands: Commands,
    config: Res<GameConfig>,
    mut dungeon_rng: ResMut<DungeonRng>,
    current_floor: Res<CurrentFloor>,
) {
    let dungeon = &config.dungeon;
    let is_last = current_floor.is_last(dungeon.max_floor);
    let floor_map = generate_bsp_floor(
        dungeon.map_width,
        dungeon.map_height,
        &mut dungeon_rng.0,
        dungeon.min_room_size,
        dungeon.max_room_size,
        is_last,
    );
    commands.insert_resource(floor_map);
}

fn render_tiles(mut commands: Commands, floor_map: Res<FloorMap>, config: Res<GameConfig>) {
    let tile_size = config.dungeon.tile_size;

    for y in 0..floor_map.height {
        for x in 0..floor_map.width {
            let kind = floor_map.tiles[(y * floor_map.width + x) as usize];
            let color = match kind {
                TileKind::Floor => Color::srgb(0.5, 0.5, 0.5),
                TileKind::Wall => Color::srgb(0.45, 0.3, 0.15),
                TileKind::Stairs => Color::srgb(0.85, 0.75, 0.2),
                TileKind::TreasureChest => Color::srgb(0.7, 0.3, 0.9),
            };

            let world_x = x as f32 * tile_size;
            let world_y = y as f32 * tile_size;

            let mut entity_commands = commands.spawn((
                Sprite::from_color(color, Vec2::splat(tile_size)),
                Transform::from_xyz(world_x, world_y, 0.0),
                Tile,
                TilePosition { x, y },
                kind,
            ));

            if kind == TileKind::Stairs {
                entity_commands.insert(Stairs);
            }
            if kind == TileKind::TreasureChest {
                entity_commands.insert(TreasureChest);
            }
        }
    }
}
