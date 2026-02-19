use bevy::prelude::*;

use crate::components::CameraFollow;
use crate::states::GameState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), spawn_camera)
            .add_systems(Update, camera_follow.run_if(in_state(GameState::Playing)));
    }
}

fn spawn_camera(mut commands: Commands, existing: Query<(), With<Camera2d>>) {
    if !existing.is_empty() {
        return;
    }
    commands.spawn(Camera2d);
}

fn camera_follow(
    player_query: Query<&Transform, With<CameraFollow>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<CameraFollow>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let target = Vec3::new(
        player_transform.translation.x,
        player_transform.translation.y,
        camera_transform.translation.z,
    );

    let lerp_speed = 5.0;
    let t = (lerp_speed * time.delta_secs()).min(1.0);
    camera_transform.translation = camera_transform.translation.lerp(target, t);
}
