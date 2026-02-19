use bevy::prelude::*;

use super::FloorEntity;

#[derive(Component)]
#[require(FloorEntity)]
pub struct Player;

#[derive(Component, Clone, Copy)]
pub struct Speed(pub f32);

#[derive(Component, Clone, Copy)]
pub struct FacingDirection(pub Vec2);

#[derive(Component)]
pub struct CameraFollow;
