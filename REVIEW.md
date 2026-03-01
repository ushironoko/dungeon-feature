# プロジェクト評価レポート

**対象**: dungeon-feature
**バージョン**: 0.1.0
**日付**: 2026-03-01
**コード規模**: 約 6,000 行 (Rust)
**テスト**: 59 テスト

---

## 総合評価

CLAUDE.md の設計哲学に対して高い準拠度を示す。ECS パターンの原則（データと振る舞いの分離）が一貫して守られており、Bevy 0.18 の機能を適切に活用している。以下、6 つの観点で詳細に評価する。

---

## 1. Trait Design — トレイト + 構造体の分離

### 評価: B+

**良い点:**
- `Plugin` トレイトによるモジュール分割が徹底されている（12 プラグイン）
- `PluginGroup` で一括登録パターンを適切に使用 (`plugins/mod.rs:19-34`)
- `Resource` トレイトの derive で ECS リソースを型安全に管理

**改善点:**
- 外部依存（オーディオ、ファイル I/O）のトレイト抽象化が未実装。CLAUDE.md で `AudioBackend` のようなトレイト抽象化を推奨しているが、現状はアセットサーバーに直接依存
- `SpriteAssets` (`resources/sprite_assets.rs:23-44`) の `enemy_handle()` / `item_handle()` はテスト可能性を考慮するとトレイトに抽出すべき
- BSP 生成ロジックは `generate_bsp_floor` 関数として公開されているが、将来的に異なるダンジョン生成アルゴリズム（Cellular Automata, Drunkard's Walk 等）を追加する際のトレイト抽象化が未準備

---

## 2. Type System Utilization — Newtype, Type State

### 評価: B

**良い点:**
- `GameState` enum による状態遷移が型レベルで保証されている (`states.rs:3-14`)
- `FloorTransitionSetup` / `PlayingSet` による SystemSet の型安全な順序制御
- `AiState` enum による敵 AI の状態遷移 (`components/enemy.rs:9-16`)
- `EnemyKind` enum による敵種別の型安全な区別
- `TileKind` enum によるタイルの型安全な表現
- `Rarity`, `EquipSlot`, `ItemKind` による型レベルの区別

**改善点:**
- **Newtype の不足**: CLAUDE.md が推奨する `PlayerId(u32)` / `EnemyId(u32)` のような ID の Newtype パターンが未使用。`Health { current: u32, max: u32 }` の `u32` が素の型のまま
- **型混同リスク**: `CurrentFloor(u32)` は Newtype だが、フロア番号、アイテムレベル、ダメージ量、HP がすべて `u32` で混同可能。`Floor(u32)`, `Level(u32)`, `Damage(u32)` のような Newtype が望ましい
- `ItemSpec` の `value: u32` が攻撃力にも防御力にも回復量にも使われる汎用フィールドで、型安全性が低い

---

## 3. Optimization Focus — 静的ディスパッチ、イテレータ活用

### 評価: A-

**良い点:**
- `ENEMY_KIND_META` が `static` スライスとして定義され、静的メタデータパターンを忠実に実装 (`plugins/combat.rs:57-97`)
- `dyn Trait` の使用がゼロ。全てジェネリクスまたは enum ディスパッチ
- System の `Query` フィルタ (`With<T>`, `Without<T>`) を適切に使用し、処理対象を限定
- BSP ノードの再帰処理が効率的で、`collect_rooms()` はスタック上の再帰で完結
- `Changed<Interaction>` による差分検出で不要な UI 更新を回避 (`plugins/menu.rs:95`, `plugins/menu.rs:398`)
- `is_changed()` による Resource の差分検出 (`plugins/item.rs:428`)

**改善点:**
- `format!()` による String アロケーションがホットパスに散在。特に `update_slot_display` (`plugins/inventory.rs:635-651`) は毎フレーム全スロットの文字列を再生成しており、`is_changed()` ガードが欲しい
- `room_indices` の `Vec` アロケーション + ソートが `spawn_enemy_batch` で毎回発生 (`plugins/enemy.rs:115-126`)。固定長配列 (`ArrayVec` 等) で代替可能
- `collect_items_for_floor` が `Vec` を返す (`plugins/transfer.rs:85-96`)。イテレータまたは固定長配列のほうが望ましい

---

## 4. Clone/Copy Strategy — 不要なクローン排除

### 評価: A

**良い点:**
- Component に `Clone, Copy` が適切に derive されている。小さな struct (`Speed(f32)`, `Attack(u32)`, `Health`, `AttackCooldown` 等) はすべて `Copy`
- `ItemSpec` が `Copy` で、装備交換時に `std::mem::replace` を使用 (`resources/player_state.rs:39`)
- `FutureTransferItem`, `PastTransferItem` も `Copy`
- font の `Handle<Font>` のみ `.clone()` を使用しているが、これは Bevy の `Handle` が `Arc` ベースで安価な clone であるため適切
- `Room` が `Copy` で BSP 処理中のコピーが軽量

**改善点:**
- `Equipment` が `Clone` だが `Copy` ではない（`Option<ItemSpec>` が 7 つ。56 bytes 程度なので `Copy` でも問題ないサイズ）。ただし将来 `ItemSpec` が拡張されると問題になる可能性があり、現状は妥当な判断
- `Inventory` の `slots: [Option<ItemSpec>; 16]` は `Copy` 可能なサイズ（約 320 bytes）だが `Clone` のみ。パフォーマンス上は問題ないが一貫性の観点で検討の余地あり

---

## 5. Memory & Allocation — コンポーネントサイズ、静的メタデータ

### 評価: A-

**良い点:**
- **固定長配列の積極活用**:
  - `Inventory { slots: [Option<ItemSpec>; 16], capacity: u8 }` — ヒープアロケーションなし
  - `TransferState { future_items: [Option<FutureTransferItem>; 32], past_items: [Option<PastTransferItem>; 32] }` — 固定長
  - `ContextMenuData { actions: [ItemAction; 6], action_count: usize }` — 固定長
- **Component サイズの最小化**: 大半の Component が 8〜16 bytes
- **静的メタデータ**: `ENEMY_KIND_META` が `&'static [EnemyKindMeta]`
- `FloorMap` の `tiles: Vec<TileKind>` は 48×48 = 2,304 要素で、フロア遷移時のみアロケーション
- `BspNode` がスタック上の再帰 + `Box` で木構造を構築し、生成完了後に破棄されるため一時アロケーションの寿命が短い

**改善点:**
- `FloorMap` が `Resource` として保持される際、`rooms: Vec<Room>` が毎フロア遷移でヒープアロケーション。部屋数は BSP の性質上最大 20 程度なので `ArrayVec<Room, 32>` 等で固定化可能
- `BspNode::collect_rooms()` が毎回 `Vec<Room>` を生成・extend。事前に容量確保するか、in-place で収集するとよい
- `format!()` の一時 String が UI 更新の度に生成される（上述の Optimization と重複）

---

## 6. Core Rust Patterns — 所有権、ライフタイム、エラーハンドリング

### 評価: A-

**良い点:**
- **パニック回避**: 全 System が `let Ok(...) = query.single() else { return; }` パターンで安全にフォールバック
- **Result 伝播の不要性**: Bevy の System は戻り値を返さないため、エラーは早期リターンで処理。これは適切
- **所有権の明確さ**: `Commands` による Entity 生成、`Res` / `ResMut` によるリソース借用が正しく使い分けられている
- **saturating 算術**: `saturating_sub` を適切に使用しオーバーフローを防止 (`plugins/combat.rs:113`, `resources/transfer_state.rs:51`)
- **境界チェック**: `FloorMap::tile_at()` がインデックス境界を確認 (`resources/dungeon.rs:40-45`)
- `let-else` パターンの一貫した使用

**改善点:**
- `enemy_count_random` (`plugins/combat.rs:147-150`) で `roll * range as f32` のキャストが `roll = 1.0` のとき `max_count + 1` を返す可能性。`(roll * range as f32).min((range - 1) as f32) as u32` でクランプすべき
- `add_future_transfer` (`plugins/transfer.rs:38-56`) で `state.charges -= 1` がアンダーフロー保護なし。`charges == 0` のチェックが呼び出し側に依存しており、`can_transfer()` がある割にこの関数内部では使っていない
- `spawn_enemy_batch` の `room.x + 1..room.x + room.width - 1` が幅 2 以下の部屋でパニックする可能性（BSP の min_room_size = 5 で実際にはガードされているが、関数単体のロバスト性として）
- `#[allow(clippy::too_many_arguments)]` が 5 箇所。Bevy System としては許容範囲だが、構造体パラメータや SystemParam の導入で改善可能

---

## アーキテクチャ上の注目点

### データと振る舞いの分離 — 準拠度: A

CLAUDE.md の「Component は純粋なデータ」「System は単一責務」の原則が高い水準で守られている。

- **Component にメソッドなし**: `combat.rs`, `dungeon.rs`, `enemy.rs`, `player.rs`, `hud.rs` の Component はすべてフィールドのみ
- **例外**: `ItemKind::equip_slot()` (`components/item.rs:38-50`) は Component ではなく enum のメソッドであり、純粋計算なので CLAUDE.md の ALLOWED ルールに合致
- **Resource のメソッド**: `Equipment::get/set`, `FloorMap::tile_at/is_walkable/has_line_of_sight_tiles`, `CurrentFloor::advance/number/is_last/reset` はすべて純粋計算で副作用なし。ALLOWED ルールに合致
- **純粋関数パターン**: `calculate_damage`, `is_in_attack_fan`, `enemy_count_random`, `slime_stats`, `compute_stat_value`, `determine_rarity`, `future_transfer_level` 等、ゲームロジックを ECS 非依存の関数として分離。テスト容易性が高い

### Event 駆動 — 疎結合設計: A-

- `DamageEvent` → `process_damage` → `DamageApplied` → フィードバック系 System のイベントチェーンが疎結合
- `EnemyDeathMessage` → アイテムドロップ + デスパーティクルの分離
- Bevy 0.18 の `Message` / `MessageWriter` / `MessageReader` を正しく活用

### Plugin 構成 — モジュラー性: A

12 Plugin が明確な責務で分離されており、依存関係が一方向。Plugin の追加・削除が容易。

---

## テスト品質

### 評価: A

- **59 テスト**が純粋関数パターンで ECS に依存しない形で書かれている
- **BSP テスト**: Flood fill による到達性検証、シード決定論テスト、境界条件テスト
- **戦闘テスト**: ダメージ計算、扇形判定、境界角度テスト
- **転送テスト**: 複利計算、past→future 変換、charges 管理、Ending 持ち帰り
- **アイテムテスト**: レアリティ判定、ステータス計算、インベントリ容量制御
- **改善点**: System レベルの統合テスト（Bevy の `App::update()` を使ったテスト）が未実装。純粋関数テストは優秀だが、System の相互作用をテストする層がない

---

## 拡張性の評価

### 新敵種の追加: 容易 (A)
`EnemyKind` enum にバリアントを追加し、`ENEMY_KIND_META` にエントリを追加するだけ。`determine_enemy_kind` のフロア出現テーブルを更新すれば完了。

### 新アイテム種の追加: 中程度 (B+)
`ItemKind` に追加し、`equip_slot()` を実装、`determine_item_kind` の確率テーブルを更新。ただし `ItemSpec.value` の汎用性により、種別固有の効果を追加する際に型の拡張が必要。

### 新チャームの追加: 容易 (A)
`CharmEffects` にフィールドを追加し、`charm_effects()` で Rarity ごとの値を設定。

### ダンジョン生成の差し替え: 中程度 (B)
`generate_bsp_floor` がトレイトではなく関数なので、別アルゴリズムに差し替えるにはリファクタが必要。`FloorMap` を返すトレイトを定義すれば容易になる。

### マルチプレイヤー対応: 困難 (C)
`Player` Component が単一想定で `.single()` が多用されている（11 箇所以上）。マルチプレイヤー化には大幅な変更が必要。ただし現時点のゲーム設計（ソロローグライク）では妥当。

---

## 具体的な改善提案（優先度順）

### P1: バグリスク

1. **`enemy_count_random` のオーバーフロー** (`plugins/combat.rs:147-150`):
   ```rust
   // 現在
   min_count + (roll * range as f32) as u32
   // 修正案
   min_count + ((roll * range as f32) as u32).min(range - 1)
   ```

2. **`add_future_transfer` の charges アンダーフロー保護** (`plugins/transfer.rs:51`):
   ```rust
   // 現在
   state.charges -= 1;
   // 修正案
   state.charges = state.charges.saturating_sub(1);
   ```

### P2: 設計改善

3. **Newtype の導入**: `Floor(u32)`, `Level(u32)`, `Damage(u32)` を導入し、型の混同を防止

4. **ダンジョン生成のトレイト化**:
   ```rust
   trait FloorGenerator {
       fn generate(&self, rng: &mut StdRng, config: &DungeonConfig, is_last: bool) -> FloorMap;
   }
   ```

5. **UI 更新の最適化**: `update_slot_display` に `if !player_state.is_changed() && !selected.is_changed() { return; }` ガードを追加

### P3: 将来的な拡張

6. **System 統合テスト**: Bevy の `App::update()` を使った統合テストの追加
7. **`ItemSpec` の型安全化**: `value` フィールドを enum で種別ごとに分離
8. **`too_many_arguments` の解消**: `SystemParam` derive による構造体パラメータ化

---

## まとめ

| 評価軸 | スコア |
|---|---|
| Trait Design | B+ |
| Type System Utilization | B |
| Optimization Focus | A- |
| Clone/Copy Strategy | A |
| Memory & Allocation | A- |
| Core Rust Patterns | A- |
| **総合** | **A-** |

CLAUDE.md の設計哲学に高い準拠度を示す、品質の高いコードベース。特に「データと振る舞いの分離」「固定長配列の活用」「純粋関数テストパターン」が優秀。Newtype の活用とトレイト抽象化の強化が主な改善余地。6,000 行のコードで 50F ローグライクの基本システムが動作しており、機能密度が高い。
