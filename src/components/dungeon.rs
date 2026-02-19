use bevy::prelude::*;

#[derive(Component, Default)]
pub struct FloorEntity;

#[derive(Component)]
#[require(FloorEntity)]
pub struct Tile;

#[derive(Component, Clone, Copy)]
pub struct TilePosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Floor,
    Wall,
    Stairs,
    TreasureChest,
}

#[derive(Component)]
pub struct Stairs;

#[derive(Component)]
#[require(FloorEntity)]
pub struct TreasureChest;
