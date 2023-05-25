//! Pivot camera allows a camera to move around a pivot point

use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};

pub const DEFAULT_KEYBOARD_SPEED: f32 = 3.;
pub const DEFAULT_KEYBOARD_ZOOM_SPEED: f32 = 12.;
pub const DEFAULT_MOUSE_SPEED: f32 = 0.3;
pub const DEFAULT_MOUSE_ZOOM_SPEED: f32 = 40.;

pub struct PivotCamerasPlugin {
    pub config: PivotCamerasConfig,
}

impl Default for PivotCamerasPlugin {
    fn default() -> Self {
        PivotCamerasPlugin {
            config: Default::default(),
        }
    }
}

impl Plugin for PivotCamerasPlugin {
    fn build(&self, app: &mut App) {
        app
            //https://discord.com/channels/691052431525675048/1094017707671822336/1094025591453401141
            .insert_resource(self.config)
            .add_system(move_cameras);
    }
}

#[derive(Resource, Copy, Clone)]
pub struct PivotCamerasConfig {
    pub keyboard_speed: f32,
    pub keyboard_zoom_speed: f32,
    pub mouse_speed: f32,
    pub mouse_zoom_speed: f32,
}

impl Default for PivotCamerasConfig {
    fn default() -> Self {
        PivotCamerasConfig {
            keyboard_speed: DEFAULT_KEYBOARD_SPEED,
            keyboard_zoom_speed: DEFAULT_KEYBOARD_ZOOM_SPEED,
            mouse_speed: DEFAULT_MOUSE_SPEED,
            mouse_zoom_speed: DEFAULT_MOUSE_ZOOM_SPEED,
        }
    }
}

#[derive(Component)]
pub struct PivotCamera {
    pub pivot: Vec3,
    pub closest: f32,
}

fn move_cameras(
    config: Res<PivotCamerasConfig>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut main_camera_query: Query<(&mut Transform, &PivotCamera)>,
) {
    let mut move_h = 0.;
    let mut move_v = 0.;
    let mut move_f = 0.;

    if mouse_input.pressed(MouseButton::Left)
        || mouse_input.pressed(MouseButton::Right)
        || mouse_input.pressed(MouseButton::Middle)
    {
        for ev in motion_evr.iter() {
            move_h -= ev.delta.x * config.mouse_speed;
            move_v -= ev.delta.y * config.mouse_speed;
        }
    }

    for ev in scroll_evr.iter() {
        match ev.unit {
            MouseScrollUnit::Line => {
                move_f -= ev.y * config.mouse_zoom_speed;
            }
            MouseScrollUnit::Pixel => {
                move_f -= ev.y * config.mouse_zoom_speed;
            }
        }
    }

    if keyboard_input.pressed(KeyCode::Left) {
        move_h += config.keyboard_speed;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        move_h -= config.keyboard_speed;
    }
    if keyboard_input.pressed(KeyCode::Up) {
        move_v += config.keyboard_speed;
    }
    if keyboard_input.pressed(KeyCode::Down) {
        move_v -= config.keyboard_speed;
    }
    if keyboard_input.pressed(KeyCode::A) {
        move_f -= config.keyboard_zoom_speed;
    }
    if keyboard_input.pressed(KeyCode::Z) {
        move_f += config.keyboard_zoom_speed;
    }

    if move_h != 0. || move_v != 0. || move_f != 0. {
        move_h *= time.delta_seconds();
        move_v *= time.delta_seconds();
        move_f *= time.delta_seconds();

        let (mut transform, pivot_camera) = main_camera_query.get_single_mut().unwrap();

        // Horizontal movement
        let local_x = transform.local_x();
        transform.rotate_around(pivot_camera.pivot, Quat::from_axis_angle(local_x, move_v));

        // Vertical movement
        // TODO (should maybe restrict to not go above?)
        transform.rotate_around(pivot_camera.pivot, Quat::from_axis_angle(Vec3::Y, move_h));

        // Zoom
        let local_z = transform.local_z();
        transform.translation += local_z * move_f;
        // Don't get too close to the pivot
        let distance = transform.translation.distance(pivot_camera.pivot);
        if distance < pivot_camera.closest {
            transform.translation -= local_z * move_f;
        }
    }
}
