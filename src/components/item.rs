use bevy::prelude::*;

use super::FloorEntity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipSlot {
    Weapon,
    Head,
    Torso,
    Legs,
    Shield,
    Charm,
    Backpack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Weapon,
    Head,
    Torso,
    Legs,
    Shield,
    Charm,
    Backpack,
    HealthPotion,
}

impl ItemKind {
    pub const fn equip_slot(&self) -> Option<EquipSlot> {
        match self {
            Self::Weapon => Some(EquipSlot::Weapon),
            Self::Head => Some(EquipSlot::Head),
            Self::Torso => Some(EquipSlot::Torso),
            Self::Legs => Some(EquipSlot::Legs),
            Self::Shield => Some(EquipSlot::Shield),
            Self::Charm => Some(EquipSlot::Charm),
            Self::Backpack => Some(EquipSlot::Backpack),
            Self::HealthPotion => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemSpec {
    pub kind: ItemKind,
    pub rarity: Rarity,
    pub level: u32,
    pub value: u32,
}

#[derive(Component)]
#[require(FloorEntity)]
pub struct Item;

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemType(pub ItemKind);

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemRarity(pub Rarity);

#[derive(Component, Debug, Clone, Copy)]
pub struct ItemLevel(pub u32);

#[derive(Component)]
pub struct DroppedItem;
