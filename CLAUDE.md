# dungeon-feature

## Project Context

- **Language**: Rust (Edition 2024)
- **Game Engine**: Bevy 0.18
- **Architecture**: ECS (Entity Component System) + データ指向設計

---

## Design Philosophy

### 核心原則: データと振る舞いの分離

rust-reviewer の設計哲学に基づき、以下を徹底する:

- **Component** = 純粋なデータ（振る舞いを持たない）
- **System** = データに対する振る舞い（データを持たない）
- **Resource** = グローバルなデータストア
- **Entity** = Component の組み合わせによる識別子

### 型システムの活用

- **Newtype**: IDや単位の混同をコンパイル時に防止する
- **Type State**: ゲーム状態の遷移をコンパイル時に保証する
- **マーカーコンポーネント**: 型レベルでエンティティを区別する

```rust
// Good: Newtype で型安全性を確保
#[derive(Component)]
struct PlayerId(u32);

#[derive(Component)]
struct EnemyId(u32);

// Good: マーカーで型レベルの区別
#[derive(Component)]
struct Friendly;

#[derive(Component)]
struct Hostile;

// Good: 状態遷移を型で表現
#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    Loading,
    Menu,
    Playing,
    Paused,
    GameOver,
}
```

### メモリとアロケーション

- Component は小さく保つ（ECS の Archetype Storage はデータレイアウトが性能に直結する）
- 固定長配列を優先し、不要な `Vec` ヒープアロケーションを避ける
- `Copy` 可能な Component は `Copy` を derive する
- 読み取り専用の共有データは `Arc` または `&'static` で保持する
- 静的メタデータパターンを活用する

```rust
// Good: 小さく Copy 可能
#[derive(Component, Clone, Copy)]
struct Speed(f32);

// Good: 固定長スロット
#[derive(Component)]
struct Inventory {
    slots: [Option<Item>; 8],
}

// Good: 静的メタデータ
static WEAPON_STATS: &[WeaponStat] = &[
    WeaponStat { name: "Sword", damage: 10.0, range: 1.5 },
    WeaponStat { name: "Bow", damage: 7.0, range: 15.0 },
];
```

### パフォーマンス意識

- Bevy の System は自動並列実行されるため、データ競合を避ける設計にする
- クエリイテレータはゼロアロケーション（そのまま活用する）
- 不要な `Clone` を排除し、参照で十分な箇所は参照を使う
- ホットパス（毎フレーム実行される System）でのアロケーションを最小化する

### トレイト設計

- プラグインやリソースの抽象化にトレイトを使う
- テスト容易性のため、外部依存（オーディオ、ファイルI/O等）はトレイトで抽象化する
- `dyn Trait` よりジェネリクスによる静的ディスパッチを優先する

```rust
// Good: トレイトでテスト可能に
trait AudioBackend: Resource {
    fn play(&self, sound: &str);
}
```

### Bevy 固有の注意点

- Bevy が所有権を管理するため、ユーザー側で複雑なライフタイムを扱う必要は少ない
- Arena アロケータ（bumpalo 等）は Bevy 内部が独自管理するため、ユーザー側では基本不要
- `dyn Trait` の出番は少ない（ECS のクエリが型を静的に解決する）

---

## Coding Standards

### MUST

- **ALWAYS** Component は純粋なデータとして設計する（メソッドを持たせない）
- **ALWAYS** 型システムで不変条件を表現する（ランタイムチェックより型チェック）
- **ALWAYS** System は単一責務にする
- **ALWAYS** エラーは `Result` で伝播し、`unwrap` はプロトタイプ段階のみ許容
- **NEVER** System 内でパニックさせない
- **NEVER** Component に振る舞い（メソッド）を持たせない
- **ALLOWED** Resource / 補助構造体の純粋計算メソッド（副作用なし）は許容する
- **NEVER** グローバルな可変状態を ECS の外に持たない

### SHOULD

- **Prefer** `Query` フィルタ（`With<T>`, `Without<T>`）で System の対象を絞る
- **Prefer** `Event` による疎結合な System 間通信
- **Prefer** `Plugin` でモジュール単位に分割する
- **Prefer** `States` でゲームフローを制御する

---

## Project Structure

```
dungeon-feature/
├── src/
│   ├── main.rs          # エントリーポイント、App builder
│   ├── plugins/         # 機能別プラグイン
│   ├── components/      # Component 定義
│   ├── systems/         # System 定義
│   ├── resources/       # Resource 定義
│   ├── events/          # Event 定義
│   └── states/          # GameState 定義
├── assets/              # ゲームアセット（画像、音声等）
├── Cargo.toml
└── CLAUDE.md
```

---

## Game Design（企画書）

### コアコンセプト

- 主人公は、とあるダンジョンを踏破することを目的とする
- ダンジョンは、複数階層で構成され、階層ごとに変化する（ローグライク）
- hack&slash である
- 2D ドット表現（Core Keeper のような縦横移動表現）
- 階を進むごとにアイテム、敵が強化されていく
- 主人公自体にレベルはなく、アイテムを装備したり、アイテムでバフをかけたりすることで強化する

### アイテム時間転送（コアメカニクス）

- 主人公はアイテムを**未来**（先の階層）、または**過去**（次回の周回）へ転送できる
- 送るアイテムはプレイヤーが選択でき、所持アイテムならなんでも選択可能
- 1 周回あたり **5 回**（未来・過去共通スタック消費）

#### 未来送り計算式

```
送った階 = S, スポーン階 = D
アイテムレベル = D + (D - S)

例1: 5階から送り → 8階にスポーン → Lv = 8 + 3 = Lv11
例2: 再送（8階から、実効Lv11）→ 11階より先にスポーン
     15階にスポーン → Lv = 15 + (15 - 8) = Lv22
例3: 再送（15階から、実効Lv22）→ 22階より先にスポーン
     30階にスポーン → Lv = 30 + (30 - 15) = Lv45
```

- 未来へ送った場合、最終階までのランダムな階層でスポーンする
- 飛んだ階層分がレベルボーナスとして加算される（複利的成長）
- 未来へ送ったアイテムを再度未来へ送ることも可能
- 再送時は現在の実効レベルに対応する階より先の階層にスポーン
- レベルは 50F キャップなし。複利的な再投資により Lv50 超えも可能

#### 過去送り計算式

```
送った階 = S
スポーン階 = 1 〜 (S-1) のランダム
アイテムレベル = スポーン階（レベルリセット）
レアリティ = 維持
```

### アイテムシステム（2 軸構造: レアリティ × レベル）

- **レアリティ**: アイテム固有の属性。深い階層ほど高レアリティが出現。初期ステータス・能力を決定
- **レベル**: 出現階層 = レベル（3 階で拾う = Lv3）。全アイテムに Lv1〜Lv50+ のステータスが存在
- 高レアリティアイテムは Lv1 時点でも低レアリティの高レベルより基礎ステータスが高い

### 装備スロット（7 枠）

| スロット | 説明 |
|---|---|
| メイン武器 | 攻撃手段 |
| 頭防具 | 防御 |
| 胴防具 | 防御 |
| 足防具 | 防御・移動系 |
| 盾 | 防御・ガード |
| チャーム | 特殊効果・パッシブ |
| バックパック | 容量管理。上位品で容量拡張 |

### ダンジョン構造

- **50F 固定**
- 階層ごとに敵・アイテムが強化
- ローグライク（階層構造はランダム生成）

### 戦闘

- **リアルタイムアクション**（Core Keeper 風 2D トップダウン）

### エンディング

- 50F 到達 → 宝箱は空 → 手持ちアイテム **1 つだけ** 持ち帰り可能
- 持ち帰りアイテム所持で周回継続可（または 1 からも選択可）

### シナリオ・テーマ

このゲームは、ユーザーが「複利」を暗黙的に体感しながら、ローグライク hack&slash を楽しむダンジョン探索型ゲーム。
主人公（ユーザー）はダンジョン内で拾ったアイテムで自身を強化しながら、ダンジョン最奥部にあると言われる秘宝を求めて彷徨う。
ユーザーは探索中に得たアイテムを、「少し先の未来へ投資」するか、「さらに先の未来（次の自分）」に投資するかを選べる。これは投資のメタファーであり、短期投資・中長期投資の選択を状況に応じて行う。
最終的に、最奥部へ到達したユーザーは宝箱を得るが、そこには何も入っていない。しかし、その時手持ちにあるアイテムを 1 つだけ持ち帰ることができる。
最奥部へ到達できるアイテムこそが、自身が投資して成長させたリターンそのものである。

---

## Implemented Features

### ゲームループ

- **状態遷移**: Loading → Menu → FloorTransition → Playing → GameOver / Ending → Menu
- **InventoryOpen**: Playing 中に I / Tab で開閉できるオーバーレイ状態
- **FloorTransition**: 階段到達時に次のフロアを BSP 生成し、エンティティをリセット

### ダンジョン生成 (BSP)

- 48×48 タイルマップ、全 50 フロア
- BSP (Binary Space Partition) によるランダム部屋 + 廊下生成
- 最初の部屋にプレイヤースポーン地点、ランダムな部屋に階段配置
- 最終フロア (50F) は階段の代わりに宝箱 (TreasureChest) を配置
- Flood fill テストでスポーン→階段/宝箱の到達性を保証

### プレイヤー

- **移動**: WASD / 矢印キー（壁衝突判定あり）
- **攻撃**: Space キー（扇形範囲攻撃、60 度・48px）
- **インタラクト**: E キー（階段昇降、宝箱オープン）
- **カメラ追従**: プレイヤー位置にカメラが追従

### 敵システム

- **配置**: 1 フロアあたり 5〜10 体をランダム配置
- **AI ステート**: Idle → Wander → Chase → Attack（視線判定 + 検知半径）
- **リスポーン**: 15 秒間隔で min_count 未満なら補充
- **ステータス**: HP / ATK / DEF / Speed がフロアに応じてスケーリング
- **装備**: 種別ごとに 0〜3 個のアイテムを装備（保証枠 + フロア依存確率枠）
- **装備ステータス反映**: スポーン時に ATK/DEF に装備ボーナスを焼き込み
- **装備ドロップ**: 撃破時に装備アイテムを 100% 確定ドロップ（30% ランダムドロップとは独立）
- **装備オーラ**: 装備を持つ敵の背後に最高レアリティ色の半透明オーラ（子エンティティ）

### 戦闘

- **ダメージ計算**: `attack.saturating_sub(defense).max(1)`（最低 1 ダメージ保証）
- **扇形攻撃判定**: `is_in_attack_fan()` で角度 + 距離判定
- **無敵時間**: 被弾後 0.5 秒間の InvincibilityTimer
- **攻撃クールダウン**: プレイヤー 0.4 秒、敵 1.0 秒
- **攻撃エフェクト**: 扇形スプライトを 0.15 秒表示
- **被弾ビジュアル**: 無敵中は半透明表示

### アイテムシステム

- **8 種類**: Weapon / Head / Torso / Legs / Shield / Charm / Backpack / HealthPotion
- **5 段階レアリティ**: Common / Uncommon / Rare / Epic / Legendary
- **フロア連動ドロップテーブル**: 深層ほど高レアリティ出現率上昇
- **レベル**: フロア番号に連動、ステータス値に影響
- **自動装備**: 未装備スロットのアイテムは拾得時に自動装備
- **ドロップ率**: 敵撃破時 30%
- **value 再計算**: `recompute_item_value()` でレベル変更後のステータス値を再計算
- **敵装備**: 敵がアイテムを装備し ATK/DEF にボーナス。撃破時に 100% 確定ドロップ

### 装備 (7 スロット)

- Weapon / Head / Torso / Legs / Shield / Charm / Backpack
- 装備により Attack / Defense が加算（レアリティ倍率 + レベルスケーリング）
- Backpack 装備でインベントリ容量が拡張

### インベントリ (16 スロット)

- **容量制御**: 基本 8 + Backpack のレアリティに応じて拡張（最大 16）
- **UI**: I / Tab で開閉、矢印キーで選択、E で装備
- **F キー**: 選択アイテムを未来の階に転送
- **P キー**: 選択アイテムを次回ランに転送（source_floor 記録）

### 時間転送システム

- **チャージ**: 1 ラン 5 回まで使用可能（未来・過去共通）
- **未来転送 (F)**:
  - アイテムをランダムな上層階に送る（実効レベルに対応する階より先）
  - スポーン時にレベルブースト適用: `Lv = D + (D - S)`（複利的成長）
  - `recompute_item_value()` で value も再計算
- **過去転送 (P)**:
  - アイテムを次回ランに保存（`source_floor` を記録）
  - `add_past_transfer()` 内で charges を減算
  - 周回開始時に `reset_for_new_run()` で past → future に変換
  - スポーン階 = 1〜(source_floor - 1) のランダム
  - レベルはスポーン階にリセット（レアリティ維持）
  - `source_floor == target_floor` により `future_transfer_level` が `D + 0 = D`
- **Ending 持ち帰り**:
  - `add_ending_carryover()` で charges を消費せず past_items に追加
  - source_floor = 1 で保存（次周回の 1F 付近にスポーン）

### UI / HUD

- **Menu 画面**: タイトル + Start ボタン
- **GameOver 画面**: 到達フロア表示 + Return to Menu ボタン
- **Ending 画面**:
  - 物語演出: 「The treasure chest is empty...」→「But you can bring one item with you.」
  - アイテムは **1 つだけ** 選択可能（EndingSelectedItem リソースで管理）
  - 選択中のボタンをハイライト表示
  - 「Continue with item」ボタン: 選択アイテムを持ち越して Menu へ
  - 「Start fresh」ボタン: アイテムなしで Menu へ（past_items クリア）
- **HUD**: HP バー、現在フロア、装備 7 スロット表示、転送チャージ残数
- **Changed\<Interaction\>**: ボタン検出を効率化

### テスト

- 88 テスト（全パス）
- 純粋関数パターン: ゲームロジックを ECS 非依存の関数として分離しユニットテスト
- テスト対象: ダメージ計算、扇形判定、敵数ランダム、スライムスケーリング、レアリティ判定、装備ステータス、転送ロジック（複利計算、レベルブースト、past→future 変換、charges 減算、Ending 持ち帰り）、敵装備生成（保証枠・確率枠・複数武器累算・フロア依存確率・レアリティ順序・ATK/DEFボーナス）、BSP 生成、Flood fill 到達性

---

## Quality Checks

```bash
cargo fmt            # フォーマット
cargo clippy         # lint
cargo check          # 型チェック
cargo test           # テスト
cargo run            # 実行
```

---

## Review

コードレビューには `rust-reviewer` エージェントを使用する。以下の6視点で評価:

1. **Trait Design** — トレイト + 構造体の分離
2. **Type System Utilization** — Newtype, Type State, Phantom Type
3. **Optimization Focus** — 静的ディスパッチ、イテレータ活用
4. **Clone/Copy Strategy** — 不要なクローン排除
5. **Memory & Allocation** — コンポーネントサイズ、静的メタデータ
6. **Core Rust Patterns** — 所有権、ライフタイム、エラーハンドリング
