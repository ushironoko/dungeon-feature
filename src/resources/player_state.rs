use bevy::prelude::*;

use crate::components::item::{EquipSlot, ItemSpec};

#[derive(Debug, Clone, Default)]
pub struct Equipment {
    pub weapon: Option<ItemSpec>,
    pub head: Option<ItemSpec>,
    pub torso: Option<ItemSpec>,
    pub legs: Option<ItemSpec>,
    pub shield: Option<ItemSpec>,
    pub charm: Option<ItemSpec>,
    pub backpack: Option<ItemSpec>,
}

impl Equipment {
    pub fn get(&self, slot: EquipSlot) -> Option<&ItemSpec> {
        match slot {
            EquipSlot::Weapon => self.weapon.as_ref(),
            EquipSlot::Head => self.head.as_ref(),
            EquipSlot::Torso => self.torso.as_ref(),
            EquipSlot::Legs => self.legs.as_ref(),
            EquipSlot::Shield => self.shield.as_ref(),
            EquipSlot::Charm => self.charm.as_ref(),
            EquipSlot::Backpack => self.backpack.as_ref(),
        }
    }

    pub fn set(&mut self, slot: EquipSlot, item: Option<ItemSpec>) -> Option<ItemSpec> {
        let target = match slot {
            EquipSlot::Weapon => &mut self.weapon,
            EquipSlot::Head => &mut self.head,
            EquipSlot::Torso => &mut self.torso,
            EquipSlot::Legs => &mut self.legs,
            EquipSlot::Shield => &mut self.shield,
            EquipSlot::Charm => &mut self.charm,
            EquipSlot::Backpack => &mut self.backpack,
        };
        std::mem::replace(target, item)
    }
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub slots: [Option<ItemSpec>; 16],
    pub capacity: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [None; 16],
            capacity: 8,
        }
    }
}

#[derive(Resource)]
pub struct PlayerState {
    pub current_hp: u32,
    pub equipment: Equipment,
    pub inventory: Inventory,
}

impl PlayerState {
    pub fn reset(&mut self, player_hp: u32) {
        self.current_hp = player_hp;
        self.equipment = Equipment::default();
        self.inventory = Inventory::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::item::{ItemKind, Rarity};

    fn make_spec(kind: ItemKind, value: u32) -> ItemSpec {
        ItemSpec {
            kind,
            rarity: Rarity::Common,
            level: 1,
            value,
        }
    }

    #[test]
    fn test_player_state_reset() {
        let mut state = PlayerState {
            current_hp: 30,
            equipment: Equipment {
                weapon: Some(make_spec(ItemKind::Weapon, 10)),
                head: Some(make_spec(ItemKind::Head, 5)),
                ..Default::default()
            },
            inventory: Inventory {
                slots: {
                    let mut s = [None; 16];
                    s[0] = Some(make_spec(ItemKind::Weapon, 1));
                    s
                },
                capacity: 8,
            },
        };
        state.reset(100);
        assert_eq!(state.current_hp, 100);
        assert!(state.equipment.weapon.is_none());
        assert!(state.equipment.head.is_none());
        assert!(state.inventory.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_equipment_get_set() {
        let mut equipment = Equipment::default();
        assert!(equipment.get(EquipSlot::Weapon).is_none());

        let spec = make_spec(ItemKind::Weapon, 10);
        let old = equipment.set(EquipSlot::Weapon, Some(spec));
        assert!(old.is_none());
        assert_eq!(equipment.get(EquipSlot::Weapon).unwrap().value, 10);

        let spec2 = make_spec(ItemKind::Weapon, 20);
        let old2 = equipment.set(EquipSlot::Weapon, Some(spec2));
        assert_eq!(old2.unwrap().value, 10);
        assert_eq!(equipment.get(EquipSlot::Weapon).unwrap().value, 20);
    }
}
