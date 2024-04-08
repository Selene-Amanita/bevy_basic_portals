//! This example illustrates how to create a simple portal,
//! it uses a single sphere that will be displayed two times on screen thanks to the portal

use bevy::prelude::*;
use bevy_basic_portals::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PortalsPlugin::MINIMAL
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-20.0, 0., 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let portal_mesh = meshes.add(Mesh::from(Rectangle::new(10., 10.)));
    commands.spawn(CreatePortalBundle {
        mesh: portal_mesh,
        // This component will be deleted and things that are needed to create the portal will be created
        create_portal: CreatePortal {
            destination: AsPortalDestination::Create(CreatePortalDestination {
                transform: Transform::from_xyz(20., 0., 0.),
                ..default()
            }),
            // Uncomment this to see the portal
            /*debug: Some(DebugPortal {
                show_window: false,
                ..default()
            }),*/
            ..default()
        },
        ..default()
    });

    let sphere_mesh = meshes.add(Mesh::from(Sphere::new(2.).mesh().uv(32, 18)));
    commands.spawn(PbrBundle {
        mesh: sphere_mesh,
        transform: Transform::from_xyz(20.,0.,-5.),
        ..default()
    });
}