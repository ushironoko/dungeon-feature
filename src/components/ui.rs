use bevy::prelude::*;

use crate::components::item::ItemSpec;

// Menu
#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct StartButton;

// GameOver
#[derive(Component)]
pub struct GameOverRoot;

#[derive(Component)]
pub struct GameOverFloorText;

#[derive(Component)]
pub struct ReturnToMenuButton;

// Ending
#[derive(Component)]
pub struct EndingRoot;

#[derive(Component)]
pub struct EndingItemSelectRoot;

#[derive(Component)]
pub struct EndingItemButton(pub usize);

#[derive(Component)]
pub struct ContinueWithItemButton;

#[derive(Component)]
pub struct StartFreshButton;

#[derive(Resource, Default)]
pub struct EndingSelectedItem(pub Option<ItemSpec>);

// Inventory
#[derive(Component)]
pub struct InventoryRoot;

#[derive(Component)]
pub struct InventorySlotNode(pub usize);

#[derive(Component)]
pub struct TransferChargeText;

#[derive(Resource, Default)]
pub struct SelectedItemIndex(pub usize);
