use bevy::prelude::*;

use crate::components::enemy::EnemyKind;
use crate::components::item::ItemKind;

#[derive(Resource)]
pub struct SpriteAssets {
    pub player: Handle<Image>,
    pub slime: Handle<Image>,
    pub bat: Handle<Image>,
    pub golem: Handle<Image>,
    pub slime_ii: Handle<Image>,
    pub bat_ii: Handle<Image>,
    pub golem_ii: Handle<Image>,
    pub item_weapon: Handle<Image>,
    pub item_head: Handle<Image>,
    pub item_torso: Handle<Image>,
    pub item_legs: Handle<Image>,
    pub item_shield: Handle<Image>,
    pub item_charm: Handle<Image>,
    pub item_backpack: Handle<Image>,
    pub item_potion: Handle<Image>,
    pub attack_sword: Handle<Image>,
}

impl SpriteAssets {
    pub fn enemy_handle(&self, kind: EnemyKind) -> &Handle<Image> {
        match kind {
            EnemyKind::Slime => &self.slime,
            EnemyKind::Bat => &self.bat,
            EnemyKind::Golem => &self.golem,
            EnemyKind::SlimeII => &self.slime_ii,
            EnemyKind::BatII => &self.bat_ii,
            EnemyKind::GolemII => &self.golem_ii,
        }
    }

    pub fn item_handle(&self, kind: ItemKind) -> &Handle<Image> {
        match kind {
            ItemKind::Weapon => &self.item_weapon,
            ItemKind::Head => &self.item_head,
            ItemKind::Torso => &self.item_torso,
            ItemKind::Legs => &self.item_legs,
            ItemKind::Shield => &self.item_shield,
            ItemKind::Charm => &self.item_charm,
            ItemKind::Backpack => &self.item_backpack,
            ItemKind::HealthPotion => &self.item_potion,
        }
    }
}

/// テクスチャがロード済みならテクスチャスプライト、未ロードなら色付き矩形を返す。
/// `color` はテクスチャ時の乗算ティント（ティント不要なら `Color::WHITE`）。
pub fn make_sprite(
    handle: &Handle<Image>,
    images: &Assets<Image>,
    fallback_color: Color,
    color: Color,
    display_size: Vec2,
) -> Sprite {
    if images.get(handle).is_some() {
        Sprite {
            image: handle.clone(),
            color,
            custom_size: Some(display_size),
            ..default()
        }
    } else {
        Sprite::from_color(fallback_color, display_size)
    }
}
