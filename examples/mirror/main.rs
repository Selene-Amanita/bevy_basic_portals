//! This example illustrates how to create a mirror

use bevy::prelude::*;
use bevy_basic_portals::*;
use helpers::{pivot_cameras, textures};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugin(PortalsPlugin::MINIMAL)
        .add_plugin(pivot_cameras::PivotCamerasPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.4,
    });
    commands.insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.2)));

    let pivot = Vec3::ZERO;
    let main_camera = commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(10., 0., 20.).looking_at(pivot, Vec3::Y),
                ..default()
            },
            pivot_cameras::PivotCamera { pivot, closest: 0. },
        ))
        .id();

    let portal_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(10., 10.))));
    let portal_transform = Transform::from_xyz(0., 0., -10.);

    commands.spawn(CreatePortalBundle {
        mesh: portal_mesh,
        create_portal: CreatePortal {
            main_camera: Some(main_camera),
            destination: AsPortalDestination::CreateMirror,
            debug: Some(DebugPortal {
                show_window: false,
                ..default()
            }),
            ..default()
        },
        portal_transform,
        ..default()
    });

    let debug_material = materials.add(textures::debug_material(&mut images, 1, None));
    let cube_mesh = meshes.add(Mesh::from(shape::Cube { size: 5. }));
    commands.spawn(PbrBundle {
        mesh: cube_mesh,
        material: debug_material,
        ..default()
    });
}
