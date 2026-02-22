use rand::Rng;
use rand::rngs::StdRng;

use crate::components::TileKind;
use crate::resources::dungeon::{FloorMap, Room};

struct BspNode {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    room: Option<Room>,
    left: Option<Box<BspNode>>,
    right: Option<Box<BspNode>>,
}

impl BspNode {
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            room: None,
            left: None,
            right: None,
        }
    }

    fn split(&mut self, rng: &mut StdRng, min_room_size: i32) {
        let min_node_size = min_room_size * 2 + 3;

        if self.width < min_node_size && self.height < min_node_size {
            return;
        }

        let split_horizontal = if self.width < min_node_size {
            true
        } else if self.height < min_node_size {
            false
        } else {
            self.height > self.width
        };

        if split_horizontal {
            let min_split = self.y + min_room_size + 1;
            let max_split = self.y + self.height - min_room_size - 1;
            if min_split >= max_split {
                return;
            }
            let split = rng.random_range(min_split..max_split);
            self.left = Some(Box::new(BspNode::new(
                self.x,
                self.y,
                self.width,
                split - self.y,
            )));
            self.right = Some(Box::new(BspNode::new(
                self.x,
                split,
                self.width,
                self.y + self.height - split,
            )));
        } else {
            let min_split = self.x + min_room_size + 1;
            let max_split = self.x + self.width - min_room_size - 1;
            if min_split >= max_split {
                return;
            }
            let split = rng.random_range(min_split..max_split);
            self.left = Some(Box::new(BspNode::new(
                self.x,
                self.y,
                split - self.x,
                self.height,
            )));
            self.right = Some(Box::new(BspNode::new(
                split,
                self.y,
                self.x + self.width - split,
                self.height,
            )));
        }

        if let Some(ref mut left) = self.left {
            left.split(rng, min_room_size);
        }
        if let Some(ref mut right) = self.right {
            right.split(rng, min_room_size);
        }
    }

    fn place_rooms(&mut self, rng: &mut StdRng, min_room_size: i32, max_room_size: i32) {
        if self.left.is_some() || self.right.is_some() {
            if let Some(ref mut left) = self.left {
                left.place_rooms(rng, min_room_size, max_room_size);
            }
            if let Some(ref mut right) = self.right {
                right.place_rooms(rng, min_room_size, max_room_size);
            }
            return;
        }

        let max_w = max_room_size.min(self.width - 2);
        let max_h = max_room_size.min(self.height - 2);

        if max_w < min_room_size || max_h < min_room_size {
            return;
        }

        let room_w = if min_room_size >= max_w {
            min_room_size
        } else {
            rng.random_range(min_room_size..=max_w)
        };
        let room_h = if min_room_size >= max_h {
            min_room_size
        } else {
            rng.random_range(min_room_size..=max_h)
        };

        let room_x = if self.x + 1 >= self.x + self.width - room_w {
            self.x + 1
        } else {
            rng.random_range((self.x + 1)..=(self.x + self.width - room_w))
        };
        let room_y = if self.y + 1 >= self.y + self.height - room_h {
            self.y + 1
        } else {
            rng.random_range((self.y + 1)..=(self.y + self.height - room_h))
        };

        self.room = Some(Room {
            x: room_x,
            y: room_y,
            width: room_w,
            height: room_h,
        });
    }

    fn collect_rooms(&self) -> Vec<Room> {
        let mut rooms = Vec::new();
        if let Some(room) = self.room {
            rooms.push(room);
        }
        if let Some(ref left) = self.left {
            rooms.extend(left.collect_rooms());
        }
        if let Some(ref right) = self.right {
            rooms.extend(right.collect_rooms());
        }
        rooms
    }

    fn get_any_room(&self) -> Option<Room> {
        if let Some(room) = self.room {
            return Some(room);
        }
        if let Some(ref left) = self.left
            && let Some(room) = left.get_any_room()
        {
            return Some(room);
        }
        if let Some(ref right) = self.right
            && let Some(room) = right.get_any_room()
        {
            return Some(room);
        }
        None
    }

    fn connect_rooms(&self, tiles: &mut [TileKind], map_width: i32) {
        let (Some(left), Some(right)) = (&self.left, &self.right) else {
            return;
        };

        left.connect_rooms(tiles, map_width);
        right.connect_rooms(tiles, map_width);

        let left_room = left.get_any_room();
        let right_room = right.get_any_room();

        if let (Some(lr), Some(rr)) = (left_room, right_room) {
            let (lx, ly) = lr.center();
            let (rx, ry) = rr.center();
            carve_l_corridor(tiles, map_width, lx, ly, rx, ry);
        }
    }
}

/// exclude_idx 以外のランダムな部屋インデックスを返す。
/// total_rooms <= 1 の場合は exclude_idx をそのまま返す。
fn select_different_room_index(rng: &mut StdRng, total_rooms: usize, exclude_idx: usize) -> usize {
    if total_rooms <= 1 {
        return exclude_idx;
    }
    let offset = rng.random_range(1..total_rooms);
    (exclude_idx + offset) % total_rooms
}

fn carve_l_corridor(tiles: &mut [TileKind], map_width: i32, x1: i32, y1: i32, x2: i32, y2: i32) {
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    for x in min_x..=max_x {
        let idx = (y1 * map_width + x) as usize;
        if idx < tiles.len() && tiles[idx] == TileKind::Wall {
            tiles[idx] = TileKind::Floor;
        }
    }

    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    for y in min_y..=max_y {
        let idx = (y * map_width + x2) as usize;
        if idx < tiles.len() && tiles[idx] == TileKind::Wall {
            tiles[idx] = TileKind::Floor;
        }
    }
}

pub fn generate_bsp_floor(
    width: i32,
    height: i32,
    rng: &mut StdRng,
    min_room_size: i32,
    max_room_size: i32,
    is_last_floor: bool,
) -> FloorMap {
    let min_room = min_room_size.max(3);
    let max_room = max_room_size.max(min_room);

    let min_required = min_room * 2 + 3;
    if width < min_required || height < min_required {
        return generate_fallback_floor(width, height, is_last_floor);
    }

    let mut tiles = vec![TileKind::Wall; (width * height) as usize];

    let mut root = BspNode::new(1, 1, width - 2, height - 2);
    root.split(rng, min_room);
    root.place_rooms(rng, min_room, max_room);

    let rooms = root.collect_rooms();

    if rooms.is_empty() {
        return generate_fallback_floor(width, height, is_last_floor);
    }

    for room in &rooms {
        for y in room.y..(room.y + room.height) {
            for x in room.x..(room.x + room.width) {
                if x >= 0 && x < width && y >= 0 && y < height {
                    tiles[(y * width + x) as usize] = TileKind::Floor;
                }
            }
        }
    }

    root.connect_rooms(&mut tiles, width);

    let spawn_idx = rng.random_range(0..rooms.len());
    let spawn_point = rooms[spawn_idx].center();

    let stairs_position = if is_last_floor {
        None
    } else {
        let stairs_idx = select_different_room_index(rng, rooms.len(), spawn_idx);
        let pos = rooms[stairs_idx].center();
        let idx = (pos.1 * width + pos.0) as usize;
        tiles[idx] = TileKind::Stairs;
        Some(pos)
    };

    if let Some(pos) = stairs_position {
        debug_assert_eq!(
            tiles[(pos.1 * width + pos.0) as usize],
            TileKind::Stairs,
            "stairs_position must point to a Stairs tile"
        );
    }

    // 最終階にはTreasureChestを配置
    let treasure_chest_position = if is_last_floor {
        let chest_idx = select_different_room_index(rng, rooms.len(), spawn_idx);
        let pos = rooms[chest_idx].center();
        let idx = (pos.1 * width + pos.0) as usize;
        tiles[idx] = TileKind::TreasureChest;
        Some(pos)
    } else {
        None
    };

    // If only 1 room and spawn == stairs, try to add a fallback room
    if rooms.len() <= 1 {
        let fallback_rooms = add_fallback_room_if_needed(&mut tiles, width, height, &rooms);
        let mut all_rooms = rooms;
        all_rooms.extend(fallback_rooms);
        return FloorMap {
            width,
            height,
            tiles,
            rooms: all_rooms,
            spawn_point,
            stairs_position,
            treasure_chest_position,
        };
    }

    FloorMap {
        width,
        height,
        tiles,
        rooms,
        spawn_point,
        stairs_position,
        treasure_chest_position,
    }
}

fn generate_fallback_floor(width: i32, height: i32, is_last_floor: bool) -> FloorMap {
    let mut tiles = vec![TileKind::Wall; (width * height) as usize];

    let room_w = (width - 4).max(3);
    let room_h = (height - 4).max(3);
    let room_x = (width - room_w) / 2;
    let room_y = (height - room_h) / 2;

    let room = Room {
        x: room_x,
        y: room_y,
        width: room_w,
        height: room_h,
    };

    for y in room.y..(room.y + room.height) {
        for x in room.x..(room.x + room.width) {
            if x >= 0 && x < width && y >= 0 && y < height {
                tiles[(y * width + x) as usize] = TileKind::Floor;
            }
        }
    }

    let spawn_point = room.center();
    let stairs_position = if is_last_floor {
        None
    } else {
        let pos = (room.x + 1, room.y + 1);
        if pos.0 >= 0 && pos.0 < width && pos.1 >= 0 && pos.1 < height {
            tiles[(pos.1 * width + pos.0) as usize] = TileKind::Stairs;
        }
        Some(pos)
    };

    let treasure_chest_position = if is_last_floor {
        let pos = (room.x + room.width - 2, room.y + room.height - 2);
        if pos.0 >= 0 && pos.0 < width && pos.1 >= 0 && pos.1 < height {
            tiles[(pos.1 * width + pos.0) as usize] = TileKind::TreasureChest;
        }
        Some(pos)
    } else {
        None
    };

    FloorMap {
        width,
        height,
        tiles,
        rooms: vec![room],
        spawn_point,
        stairs_position,
        treasure_chest_position,
    }
}

fn add_fallback_room_if_needed(
    tiles: &mut [TileKind],
    _width: i32,
    _height: i32,
    _existing_rooms: &[Room],
) -> Vec<Room> {
    // Already have corridors from BSP, no additional rooms needed for single-room case
    let _ = tiles;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make_rng(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }

    #[test]
    fn test_generates_multiple_rooms() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, false);
        assert!(
            floor.rooms.len() >= 2,
            "Expected >= 2 rooms, got {}",
            floor.rooms.len()
        );
    }

    #[test]
    fn test_spawn_and_stairs_different() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, false);
        let stairs = floor
            .stairs_position
            .expect("Non-last floor should have stairs");
        assert_ne!(
            floor.spawn_point, stairs,
            "Spawn and stairs should be at different positions"
        );
    }

    #[test]
    fn test_last_floor_no_stairs() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, true);
        assert!(
            floor.stairs_position.is_none(),
            "Last floor should have no stairs"
        );
    }

    #[test]
    fn test_deterministic_with_seed() {
        let mut rng1 = make_rng(123);
        let floor1 = generate_bsp_floor(48, 48, &mut rng1, 5, 15, false);
        let mut rng2 = make_rng(123);
        let floor2 = generate_bsp_floor(48, 48, &mut rng2, 5, 15, false);
        assert_eq!(
            floor1.tiles, floor2.tiles,
            "Same seed should produce same floor"
        );
        assert_eq!(floor1.spawn_point, floor2.spawn_point);
        assert_eq!(floor1.stairs_position, floor2.stairs_position);
    }

    #[test]
    fn test_all_rooms_accessible() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, false);

        let mut visited = vec![false; (floor.width * floor.height) as usize];
        let mut stack = vec![floor.spawn_point];

        while let Some((x, y)) = stack.pop() {
            if x < 0 || y < 0 || x >= floor.width || y >= floor.height {
                continue;
            }
            let idx = (y * floor.width + x) as usize;
            if visited[idx] {
                continue;
            }
            match floor.tiles[idx] {
                TileKind::Floor | TileKind::Stairs | TileKind::TreasureChest => {}
                TileKind::Wall => continue,
            }
            visited[idx] = true;
            stack.push((x - 1, y));
            stack.push((x + 1, y));
            stack.push((x, y - 1));
            stack.push((x, y + 1));
        }

        for (i, tile) in floor.tiles.iter().enumerate() {
            if matches!(
                tile,
                TileKind::Floor | TileKind::Stairs | TileKind::TreasureChest
            ) {
                assert!(
                    visited[i],
                    "Tile at ({}, {}) is not reachable from spawn",
                    i as i32 % floor.width,
                    i as i32 / floor.width
                );
            }
        }
    }

    #[test]
    fn test_spawn_on_floor_tile() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, false);
        let (sx, sy) = floor.spawn_point;
        let tile = floor.tile_at(sx, sy);
        assert_eq!(
            tile,
            Some(TileKind::Floor),
            "Spawn point must be a Floor tile"
        );
    }

    #[test]
    fn test_no_out_of_bounds_tiles() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, false);
        assert_eq!(
            floor.tiles.len(),
            (floor.width * floor.height) as usize,
            "Tile count must match map dimensions"
        );
        for room in &floor.rooms {
            assert!(room.x >= 0, "Room x out of bounds");
            assert!(room.y >= 0, "Room y out of bounds");
            assert!(
                room.x + room.width <= floor.width,
                "Room extends beyond map width"
            );
            assert!(
                room.y + room.height <= floor.height,
                "Room extends beyond map height"
            );
        }
    }

    #[test]
    fn test_minimum_map_size_fallback() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(8, 8, &mut rng, 5, 15, false);
        assert!(
            !floor.rooms.is_empty(),
            "Fallback should create at least 1 room"
        );
        assert!(
            floor.stairs_position.is_some(),
            "Non-last floor should have stairs"
        );

        let (sx, sy) = floor.spawn_point;
        assert!(
            floor.is_walkable(sx, sy),
            "Spawn point must be walkable in fallback"
        );
    }

    #[test]
    fn test_last_floor_has_treasure_chest() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, true);
        assert!(
            floor.treasure_chest_position.is_some(),
            "Last floor should have a treasure chest"
        );
        let (cx, cy) = floor.treasure_chest_position.unwrap();
        assert_eq!(
            floor.tile_at(cx, cy),
            Some(TileKind::TreasureChest),
            "treasure_chest_position must point to a TreasureChest tile"
        );
    }

    #[test]
    fn test_treasure_chest_reachable_from_spawn() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, true);
        let chest_pos = floor
            .treasure_chest_position
            .expect("Should have treasure chest");

        let mut visited = vec![false; (floor.width * floor.height) as usize];
        let mut stack = vec![floor.spawn_point];

        while let Some((x, y)) = stack.pop() {
            if x < 0 || y < 0 || x >= floor.width || y >= floor.height {
                continue;
            }
            let idx = (y * floor.width + x) as usize;
            if visited[idx] {
                continue;
            }
            match floor.tiles[idx] {
                TileKind::Floor | TileKind::Stairs | TileKind::TreasureChest => {}
                TileKind::Wall => continue,
            }
            visited[idx] = true;
            stack.push((x - 1, y));
            stack.push((x + 1, y));
            stack.push((x, y - 1));
            stack.push((x, y + 1));
        }

        let chest_idx = (chest_pos.1 * floor.width + chest_pos.0) as usize;
        assert!(
            visited[chest_idx],
            "TreasureChest at ({}, {}) must be reachable from spawn",
            chest_pos.0, chest_pos.1,
        );
    }

    #[test]
    fn test_non_last_floor_no_treasure_chest() {
        let mut rng = make_rng(42);
        let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, false);
        assert!(
            floor.treasure_chest_position.is_none(),
            "Non-last floor should not have a treasure chest"
        );
    }

    #[test]
    fn test_select_different_room_index_two_rooms() {
        let mut rng = make_rng(99);
        // 2部屋: exclude=0 → 必ず1, exclude=1 → 必ず0
        assert_eq!(select_different_room_index(&mut rng, 2, 0), 1);
        assert_eq!(select_different_room_index(&mut rng, 2, 1), 0);
    }

    #[test]
    fn test_select_different_room_index_one_room() {
        let mut rng = make_rng(99);
        // 1部屋: exclude をそのまま返す
        assert_eq!(select_different_room_index(&mut rng, 1, 0), 0);
    }

    #[test]
    fn test_select_different_room_index_many_rooms() {
        let mut rng = make_rng(42);
        let total = 10;
        for exclude in 0..total {
            let result = select_different_room_index(&mut rng, total, exclude);
            assert_ne!(result, exclude, "Must select a different room");
            assert!(result < total, "Index must be in range");
        }
    }

    #[test]
    fn test_randomized_spawn_deterministic_with_seed() {
        let mut rng1 = make_rng(777);
        let floor1 = generate_bsp_floor(48, 48, &mut rng1, 5, 15, false);
        let mut rng2 = make_rng(777);
        let floor2 = generate_bsp_floor(48, 48, &mut rng2, 5, 15, false);
        assert_eq!(floor1.spawn_point, floor2.spawn_point);
        assert_eq!(floor1.stairs_position, floor2.stairs_position);
    }

    #[test]
    fn test_treasure_chest_not_at_spawn() {
        // 複数部屋がある場合、宝箱はスポーンと異なる位置
        for seed in 0..20u64 {
            let mut rng = make_rng(seed);
            let floor = generate_bsp_floor(48, 48, &mut rng, 5, 15, true);
            if floor.rooms.len() > 1 {
                let chest = floor
                    .treasure_chest_position
                    .expect("Last floor should have chest");
                assert_ne!(
                    floor.spawn_point, chest,
                    "Treasure chest should not be at spawn (seed={})",
                    seed
                );
            }
        }
    }
}
