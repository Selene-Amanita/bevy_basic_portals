//! This example illustrates how to create a mirror

use bevy::prelude::*;
use bevy_basic_portals::*;

#[path = "../../helpers/pivot_cameras.rs"]
mod pivot_cameras;
#[path = "../../helpers/textures.rs"]
mod textures;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            PortalsPlugin::MINIMAL,
            pivot_cameras::PivotCamerasPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 400.,
    });
    commands.insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.2)));

    // Camera
    let pivot = Vec3::ZERO;
    let main_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(10., 0., 20.).looking_at(pivot, Vec3::Y),
            pivot_cameras::PivotCamera {
                pivot,
                closest: 0.,
                ..default()
            },
        ))
        .id();

    // Cube
    let debug_material = materials.add(textures::debug_material(&mut images, 1, None));
    let cube_mesh = meshes.add(Cuboid::new(5., 5., 5.));
    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(debug_material),
    ));

    // Torus
    let torus_mesh = meshes.add(Torus::new(2.25, 2.75));
    let torus = commands
        .spawn((
            Mesh3d(torus_mesh),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(0., 0., -5.),
        ))
        .id();

    // Mirror
    let portal_mesh = meshes.add(Rectangle::new(10., 10.));
    let portal_transform = Transform::from_xyz(0., 0., -10.);
    let mut mirror = commands.spawn((
        CreatePortal {
            main_camera: Some(main_camera),
            destination: AsPortalDestination::CreateMirror,
            debug: Some(DebugPortal {
                show_window: false,
                ..default()
            }),
            ..default()
        },
        Mesh3d(portal_mesh),
        portal_transform,
    ));

    mirror.add_child(torus);
}
