use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Playing,
    Paused,
    FloorTransition,
    GameOver,
    Ending,
    InventoryOpen,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloorTransitionSetup {
    Cleanup,
    AdvanceFloor,
    GenerateFloor,
    RenderTiles,
    SpawnEntities,
    Complete,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlayingSet {
    Player,
    Enemy,
    Combat,
    Item,
    PostCombat,
}
