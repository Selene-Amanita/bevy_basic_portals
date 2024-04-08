//! Pivot camera allows a camera to move around a pivot point

use bevy::{
    prelude::*,
    input::mouse::{MouseMotion, MouseWheel, MouseScrollUnit},
};

pub const DEFAULT_KEYBOARD_SPEED: f32 = 3.;
pub const DEFAULT_KEYBOARD_ZOOM_SPEED: f32 = 12.;
pub const DEFAULT_MOUSE_SPEED: f32 = 0.3;
pub const DEFAULT_MOUSE_ZOOM_SPEED: f32 = 40.;

pub struct PivotCamerasPlugin {
    pub config: Option<PivotCamerasConfig>
}

impl Default for PivotCamerasPlugin {
    fn default() -> Self {
        PivotCamerasPlugin {
            config: Default::default()
        }
    }
}

impl Plugin for PivotCamerasPlugin {
    fn build(&self, app: &mut App) {
        if let Some(config) = self.config {
            app.insert_resource(config);
        } else {
            app.init_resource::<PivotCamerasConfig>();
        }
        app.add_systems(Update, update_pivot_cameras);
    }
}

#[derive(Resource, Copy, Clone)]
pub struct PivotCamerasConfig {
    pub keyboard_speed: f32,
    pub keyboard_zoom_speed: f32,
    pub mouse_speed: f32,
    pub mouse_zoom_speed: f32,
    pub keyboard_left_key: KeyCode,
    pub keyboard_right_key: KeyCode,
    pub keyboard_up_key: KeyCode,
    pub keyboard_down_key: KeyCode,
    pub keyboard_forward_key: KeyCode,
    pub keyboard_backward_key: KeyCode,
}

impl Default for PivotCamerasConfig {
    fn default() -> Self {
        PivotCamerasConfig { 
            keyboard_speed: DEFAULT_KEYBOARD_SPEED,
            keyboard_zoom_speed: DEFAULT_KEYBOARD_ZOOM_SPEED,
            mouse_speed: DEFAULT_MOUSE_SPEED,
            mouse_zoom_speed: DEFAULT_MOUSE_ZOOM_SPEED,
            keyboard_left_key: KeyCode::ArrowLeft,
            keyboard_right_key: KeyCode::ArrowRight,
            keyboard_up_key: KeyCode::ArrowUp,
            keyboard_down_key: KeyCode::ArrowDown,
            keyboard_forward_key: KeyCode::KeyZ,
            keyboard_backward_key: KeyCode::KeyA,
        }
    }
}

#[derive(Component)]
pub struct PivotCamera {
    pub pivot: Vec3,
    pub closest: f32,
    pub mouse_controlled: bool,
    pub keyboard_controlled: bool,
}

impl Default for PivotCamera {
    fn default() -> Self {
        Self {
            pivot: Vec3::ZERO,
            closest: 0.1,
            mouse_controlled: true,
            keyboard_controlled: true,
        }
    }
}

#[derive(Default, PartialEq, Clone)]
struct MoveForDevice {
    h: f32,
    v: f32,
    f: f32,
}
impl Eq for MoveForDevice {}
impl std::ops::MulAssign<f32> for MoveForDevice {
    fn mul_assign(&mut self, rhs: f32) {
        self.h *= rhs;
        self.v *= rhs;
        self.f *= rhs;
    }
}
impl std::ops::AddAssign for MoveForDevice {
    fn add_assign(&mut self, rhs: Self) {
        self.h += rhs.h;
        self.v += rhs.v;
        self.f += rhs.f;
    }
}

#[derive(Default, PartialEq, Eq)]
struct Move {
    mouse: MoveForDevice,
    keyboard: MoveForDevice,
}

fn update_pivot_cameras(
    config: Res<PivotCamerasConfig>,
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut pivot_camera_query: Query<(&mut Transform, &PivotCamera)>
) {
    let still = Move::default();
    let mut mov = Move::default();

    if mouse_input.pressed(MouseButton::Left) || mouse_input.pressed(MouseButton::Right) || mouse_input.pressed(MouseButton::Middle) {
        for ev in motion_evr.read() {
            mov.mouse.h -= ev.delta.x * config.mouse_speed;
            mov.mouse.v -= ev.delta.y * config.mouse_speed;
        }
    }

    for ev in scroll_evr.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                mov.mouse.f -= ev.y * config.mouse_zoom_speed;
            }
            MouseScrollUnit::Pixel => {
                mov.mouse.f -= ev.y * config.mouse_zoom_speed;
            }
        }
    }

    if keyboard_input.pressed(config.keyboard_left_key) {
        mov.keyboard.h += config.keyboard_speed;
    }
    if keyboard_input.pressed(config.keyboard_right_key) {
        mov.keyboard.h -= config.keyboard_speed;
    }
    if keyboard_input.pressed(config.keyboard_up_key) {
        mov.keyboard.v += config.keyboard_speed;
    }
    if keyboard_input.pressed(config.keyboard_down_key) {
        mov.keyboard.v -= config.keyboard_speed;
    }
    if keyboard_input.pressed(config.keyboard_forward_key) {
        mov.keyboard.f -= config.keyboard_zoom_speed;
    }
    if keyboard_input.pressed(config.keyboard_backward_key) {
        mov.keyboard.f += config.keyboard_zoom_speed;
    }

    if mov != still {
        mov.keyboard *= time.delta_seconds();
        mov.mouse *= time.delta_seconds();

        for (mut transform, pivot_camera) in pivot_camera_query.iter_mut() {
            let mut move_cam = MoveForDevice::default();

            if pivot_camera.mouse_controlled {
                move_cam += mov.mouse.clone();
            }

            if pivot_camera.keyboard_controlled {
                move_cam += mov.keyboard.clone();
            }

            // Vertical movement
            // TODO (should maybe restrict to not go above?)
            let local_x = transform.local_x();
            transform.rotate_around(pivot_camera.pivot, Quat::from_axis_angle(*local_x, move_cam.v));
    
            // Horizontal movement
            transform.rotate_around(pivot_camera.pivot, Quat::from_axis_angle(Vec3::Y, move_cam.h));
    
            // Zoom
            let local_z = transform.local_z();
            transform.translation += local_z * move_cam.f;
            // Don't get too close to the pivot
            let distance = transform.translation.distance(pivot_camera.pivot);
            if distance < pivot_camera.closest {
                transform.translation -= local_z * move_cam.f;
            }
        }
    }
}