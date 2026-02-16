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
