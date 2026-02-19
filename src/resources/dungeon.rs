use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::components::TileKind;

#[derive(Clone, Copy)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Room {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    pub fn intersects(&self, other: &Room) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

#[derive(Resource)]
pub struct FloorMap {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<TileKind>,
    pub rooms: Vec<Room>,
    pub spawn_point: (i32, i32),
    pub stairs_position: Option<(i32, i32)>,
    pub treasure_chest_position: Option<(i32, i32)>,
}

impl FloorMap {
    pub fn tile_at(&self, x: i32, y: i32) -> Option<TileKind> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some(self.tiles[(y * self.width + x) as usize])
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        matches!(
            self.tile_at(x, y),
            Some(TileKind::Floor | TileKind::Stairs | TileKind::TreasureChest)
        )
    }

    /// Bresenham LOS（タイル座標ベース）: from/to 間の全タイルが walkable なら true
    pub fn has_line_of_sight_tiles(&self, from: (i32, i32), to: (i32, i32)) -> bool {
        let (mut x0, mut y0) = from;
        let (x1, y1) = to;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if !self.is_walkable(x0, y0) {
                return false;
            }
            if x0 == x1 && y0 == y1 {
                return true;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

/// ピクセル→タイル座標変換
pub fn pixel_to_tile(pos: Vec2, tile_size: f32) -> (i32, i32) {
    (
        (pos.x / tile_size).round() as i32,
        (pos.y / tile_size).round() as i32,
    )
}

/// 壁衝突を考慮した移動後の座標を返す（player/enemy 共用）
pub fn apply_movement_with_collision(
    current: Vec3,
    velocity: Vec2,
    floor_map: &FloorMap,
    tile_size: f32,
) -> Vec3 {
    let mut result = current;

    // X 方向チェック
    let new_x = current.x + velocity.x;
    let tile_after_x = (new_x / tile_size).round() as i32;
    let current_tile_y = (current.y / tile_size).round() as i32;
    if floor_map.is_walkable(tile_after_x, current_tile_y) {
        result.x = new_x;
    }

    // Y 方向チェック（X 更新後の位置を基準にする）
    let current_tile_x = (result.x / tile_size).round() as i32;
    let new_y = current.y + velocity.y;
    let tile_after_y = (new_y / tile_size).round() as i32;
    if floor_map.is_walkable(current_tile_x, tile_after_y) {
        result.y = new_y;
    }

    result
}

#[derive(Resource, Default)]
pub struct CurrentFloor(u32);

impl CurrentFloor {
    pub fn advance(&mut self) {
        self.0 += 1;
    }

    pub fn number(&self) -> u32 {
        self.0
    }

    pub fn is_last(&self, max_floor: u32) -> bool {
        self.0 >= max_floor
    }

    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

#[derive(Resource)]
pub struct DungeonRng(pub StdRng);

impl Default for DungeonRng {
    fn default() -> Self {
        Self(StdRng::from_os_rng())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_floor_map(width: i32, height: i32, tiles: Vec<TileKind>) -> FloorMap {
        FloorMap {
            width,
            height,
            tiles,
            rooms: vec![],
            spawn_point: (0, 0),
            stairs_position: None,
            treasure_chest_position: None,
        }
    }

    #[test]
    fn test_current_floor_reset() {
        let mut floor = CurrentFloor::default();
        floor.advance();
        floor.advance();
        assert_eq!(floor.number(), 2);
        floor.reset();
        assert_eq!(floor.number(), 0);
    }

    #[test]
    fn test_is_walkable_treasure_chest() {
        let tiles = vec![TileKind::TreasureChest];
        let map = make_floor_map(1, 1, tiles);
        assert!(map.is_walkable(0, 0));
    }

    #[test]
    fn test_line_of_sight_open_space() {
        // 3x3 の全 Floor マップ
        let map = make_floor_map(3, 3, vec![TileKind::Floor; 9]);
        assert!(map.has_line_of_sight_tiles((0, 0), (2, 2)));
        assert!(map.has_line_of_sight_tiles((0, 0), (2, 0)));
        assert!(map.has_line_of_sight_tiles((0, 0), (0, 2)));
    }

    #[test]
    fn test_line_of_sight_blocked_by_wall() {
        // 3x3 マップ、中央が壁
        let mut tiles = vec![TileKind::Floor; 9];
        tiles[4] = TileKind::Wall; // (1,1) が壁
        let map = make_floor_map(3, 3, tiles);
        assert!(!map.has_line_of_sight_tiles((0, 0), (2, 2)));
    }

    #[test]
    fn test_line_of_sight_same_position() {
        let map = make_floor_map(3, 3, vec![TileKind::Floor; 9]);
        assert!(map.has_line_of_sight_tiles((1, 1), (1, 1)));
    }

    #[test]
    fn test_pixel_to_tile_rounding() {
        assert_eq!(pixel_to_tile(Vec2::new(32.0, 64.0), 32.0), (1, 2));
        assert_eq!(pixel_to_tile(Vec2::new(48.0, 48.0), 32.0), (2, 2));
        assert_eq!(pixel_to_tile(Vec2::new(15.0, 15.0), 32.0), (0, 0));
        assert_eq!(pixel_to_tile(Vec2::new(16.0, 16.0), 32.0), (1, 1));
    }

    #[test]
    fn test_apply_movement_collision_blocks_wall() {
        // 3x1: Floor, Wall, Floor
        let tiles = vec![TileKind::Floor, TileKind::Wall, TileKind::Floor];
        let map = make_floor_map(3, 1, tiles);
        let current = Vec3::new(0.0, 0.0, 1.0);
        let velocity = Vec2::new(32.0, 0.0); // 壁方向へ移動
        let result = apply_movement_with_collision(current, velocity, &map, 32.0);
        // X 方向は壁でブロック
        assert_eq!(result.x, 0.0);
    }

    #[test]
    fn test_apply_movement_collision_allows_floor() {
        // 3x1: Floor, Floor, Floor
        let tiles = vec![TileKind::Floor; 3];
        let map = make_floor_map(3, 1, tiles);
        let current = Vec3::new(0.0, 0.0, 1.0);
        let velocity = Vec2::new(32.0, 0.0);
        let result = apply_movement_with_collision(current, velocity, &map, 32.0);
        assert_eq!(result.x, 32.0);
    }
}
