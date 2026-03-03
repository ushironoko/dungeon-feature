use bevy::prelude::*;
use rand::Rng;

use crate::components::item::{ItemKind, ItemSpec, Rarity};

#[derive(Debug, Clone, Copy)]
pub struct FutureTransferItem {
    pub spec: ItemSpec,
    pub source_floor: u32,
    pub target_floor: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct PastTransferItem {
    pub spec: ItemSpec,
    pub source_floor: u32,
}

#[derive(Resource)]
pub struct TransferState {
    pub charges: u32,
    pub future_items: [Option<FutureTransferItem>; 32],
    pub past_items: [Option<PastTransferItem>; 32],
}

impl TransferState {
    pub fn new(charges: u32) -> Self {
        Self {
            charges,
            future_items: [None; 32],
            past_items: [None; 32],
        }
    }

    /// 周回開始時: charges リセット + past_items → future_items 変換（レベルリセット）
    pub fn reset_for_new_run(&mut self, charges: u32, rng: &mut impl Rng) {
        self.charges = charges;
        self.future_items = [None; 32];

        for i in 0..self.past_items.len() {
            if self.past_items[i].is_none() {
                continue;
            }

            let has_space = self.future_items.iter().any(|s| s.is_none());
            if !has_space {
                break;
            }

            if let Some(item) = self.past_items[i] {
                let max_floor = item.source_floor.saturating_sub(1).max(1);
                let spawn_floor = if max_floor <= 1 {
                    1
                } else {
                    rng.random_range(1..=max_floor)
                };
                let mut resolved_spec = item.spec;
                resolved_spec.level = spawn_floor;

                for future_slot in &mut self.future_items {
                    if future_slot.is_none() {
                        *future_slot = Some(FutureTransferItem {
                            spec: resolved_spec,
                            source_floor: spawn_floor,
                            target_floor: spawn_floor,
                        });
                        break;
                    }
                }
                self.past_items[i] = None;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArrivedItemInfo {
    pub kind: ItemKind,
    pub rarity: Rarity,
    pub level: u32,
}

#[derive(Resource, Default)]
pub struct TransferArrivalNotice {
    pub items: [Option<ArrivedItemInfo>; 8],
}

impl TransferArrivalNotice {
    pub fn clear(&mut self) {
        self.items = [None; 8];
    }

    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|s| s.is_none())
    }
}
