use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteCategory {
    Enemy,
    Item,
}

#[derive(Debug, Clone)]
pub struct SpriteSpec {
    pub name: &'static str,
    pub category: SpriteCategory,
    pub description: &'static str,
}

impl SpriteSpec {
    pub fn asset_path(&self) -> String {
        let dir = match self.category {
            SpriteCategory::Enemy => "sprites/enemies",
            SpriteCategory::Item => "sprites/items",
        };
        format!("{}/{}.png", dir, self.name)
    }

    pub fn full_path(&self, project_root: &Path) -> PathBuf {
        project_root.join("assets").join(self.asset_path())
    }
}

pub static SPRITE_MANIFEST: &[SpriteSpec] = &[
    SpriteSpec {
        name: "bat",
        category: SpriteCategory::Enemy,
        description: "紫色の飛行コウモリ、赤い目",
    },
    SpriteSpec {
        name: "golem",
        category: SpriteCategory::Enemy,
        description: "灰色の岩ゴーレム、光る目",
    },
    SpriteSpec {
        name: "slime_ii",
        category: SpriteCategory::Enemy,
        description: "大型の深緑スライム、体内に光る核、トゲのある表面",
    },
    SpriteSpec {
        name: "bat_ii",
        category: SpriteCategory::Enemy,
        description: "赤紫の炎を纏うコウモリ、光る翼、鋭い牙",
    },
    SpriteSpec {
        name: "golem_ii",
        category: SpriteCategory::Enemy,
        description: "青い水晶で構成されたゴーレム、発光するコア、角張った体",
    },
    SpriteSpec {
        name: "weapon",
        category: SpriteCategory::Item,
        description: "短剣/剣、金属刃+革柄",
    },
    SpriteSpec {
        name: "head",
        category: SpriteCategory::Item,
        description: "ヘルメット/帽子",
    },
    SpriteSpec {
        name: "torso",
        category: SpriteCategory::Item,
        description: "胸鎧/革ベスト",
    },
    SpriteSpec {
        name: "legs",
        category: SpriteCategory::Item,
        description: "ブーツ/脛当て",
    },
    SpriteSpec {
        name: "shield",
        category: SpriteCategory::Item,
        description: "丸盾/木製+金属縁",
    },
    SpriteSpec {
        name: "charm",
        category: SpriteCategory::Item,
        description: "魔法のアミュレット、宝石+鎖",
    },
    SpriteSpec {
        name: "backpack",
        category: SpriteCategory::Item,
        description: "革の小型バッグ",
    },
    SpriteSpec {
        name: "potion",
        category: SpriteCategory::Item,
        description: "赤い回復ポーション瓶",
    },
];

pub fn find_sprite(name: &str) -> Option<&'static SpriteSpec> {
    SPRITE_MANIFEST.iter().find(|s| s.name == name)
}

pub fn find_missing_sprites(project_root: &Path) -> Vec<&'static SpriteSpec> {
    SPRITE_MANIFEST
        .iter()
        .filter(|spec| !spec.full_path(project_root).exists())
        .collect()
}
