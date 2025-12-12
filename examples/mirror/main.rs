//! This example illustrates how to create a mirror

use bevy::prelude::*;
use bevy_basic_portals::*;
use bevy_color::palettes::basic::*;

#[path = "../../helpers/pivot_cameras.rs"]
mod pivot_cameras;

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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 400.,
        affects_lightmapped_meshes: true,
    });
    commands.insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.2)));

    // Camera
    let pivot = Vec3::ZERO;
    let camera_transform = Transform::from_xyz(10., 5., 20.).looking_at(pivot, Vec3::Y);
    let main_camera = commands
        .spawn((
            Camera3d::default(),
            camera_transform,
            pivot_cameras::PivotCamera {
                pivot,
                closest: 0.,
                ..default()
            },
        ))
        .id();

    // Cubes
    let cube_mesh = meshes.add(Cuboid::new(2., 2., 2.));
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(materials.add(Color::Srgba(BLUE))),
        Transform::from_xyz(2., -2., 0.),
    ));
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(materials.add(Color::Srgba(YELLOW))),
        Transform::from_xyz(2., 2., 0.),
    ));
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(materials.add(Color::Srgba(RED))),
        Transform::from_xyz(-2., 2., 0.),
    ));
    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(materials.add(Color::Srgba(GREEN))),
        Transform::from_xyz(-2., -2., 0.),
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
    //portal_transform.rotate(Quat::from_axis_angle(Vec3::Z, FRAC_PI_6));
    let mut mirror = commands.spawn((
        CreatePortal {
            main_camera: Some(main_camera),
            destination: PortalDestinationSource::CreateMirror,
            debug: Some(DebugPortal {
                // Set to true to see what the portal camera really sees
                show_window: false,
                ..default()
            }),
            // Uncomment the following two lines to have a double-sided mirror
            //cull_mode: None,
            //portal_mode: PortalMode::MaskedImageHalfSpaceFrustum((None, true)),
            ..default()
        },
        Mesh3d(portal_mesh),
        portal_transform,
    ));
    mirror.add_child(torus);
}
