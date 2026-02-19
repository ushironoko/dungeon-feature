use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin, WindowResolution};

use dungeon_feature::plugins::DungeonFeaturePlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dungeon Feature".to_string(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DungeonFeaturePlugins)
        .run();
}
