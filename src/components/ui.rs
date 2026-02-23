use bevy::prelude::*;

use crate::components::item::{EquipSlot, ItemSpec};

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

#[derive(Component)]
pub struct EquipmentPanel;

#[derive(Component)]
pub struct EquipmentSlotNode(pub EquipSlot);

#[derive(Component)]
pub struct InventoryPanel;

#[derive(Component)]
pub struct ContextMenu;

#[derive(Component)]
pub struct ContextMenuItem(pub ItemAction);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    Equip,
    Use,
    SendFuture,
    SendPast,
    Discard,
    Close,
}

/// None = Browse モード、Some = Menu モード
#[derive(Resource, Default)]
pub struct ContextMenuState(pub Option<ContextMenuData>);

pub struct ContextMenuData {
    pub selected: usize,
    pub actions: [ItemAction; 6],
    pub action_count: usize,
}
