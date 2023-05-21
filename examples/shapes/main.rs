//! This example illustrates using portals that are not simple planes
//! 
//! On top you can see portals, at the bottom in the center is where their destination is,
//! and around are shapes that you can also see through the portal with the correct orientation

use bevy::{prelude::*, render::render_resource::Face};
use bevy_basic_portals::*;
use helpers::{textures, pivot_cameras};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PortalsPlugin{check_create: portals::PortalsCheckMode::CheckAfterStartup})
        .add_plugin(pivot_cameras::PivotCamerasPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    // Camera
    let pivot = Vec3::ZERO;
    let main_camera = commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0., 20.0).looking_at(pivot, Vec3::Y),
            ..default()
        },
        pivot_cameras::PivotCamera {
            pivot,
            closest: 0.
        },
    )).id();

    // Lights
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.1,
    });

    commands.insert_resource(ClearColor(Color::rgb(0., 0., 0.)));

    // Sphere
    let debug_material = materials.add(textures::debug_material(&mut images, 3, Some(Face::Back)));
    let sphere_mesh = meshes.add(Mesh::from(shape::UVSphere{radius: 2.5, ..default()}));
    setup_object_and_portal(&mut commands, main_camera, sphere_mesh, debug_material.clone(), Transform::from_xyz(10.,0.,0.), Some(Face::Back));

    // Cube
    let debug_material = materials.add(textures::debug_material(&mut images, 2, Some(Face::Back)));
    let cube_mesh = meshes.add(Mesh::from(shape::Cube::new(5.)));
    setup_object_and_portal(&mut commands, main_camera, cube_mesh, debug_material.clone(), Transform::from_xyz(-10.,0.,0.), Some(Face::Back));

    // double-sided Quad
    let debug_material = materials.add(textures::debug_material(&mut images, 1, None));
    let quad_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(5.,5.))));
    setup_object_and_portal(&mut commands, main_camera, quad_mesh, debug_material.clone(), Transform::from_xyz(0.,0.,-10.), None);
}

fn setup_object_and_portal(
    commands: &mut Commands,
    main_camera: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    portal_transform: Transform,
    cull_mode: Option<Face>
) {
    //Object
    let mut object_transform = portal_transform.clone();
    object_transform.translation.y -= 10.;
    commands.spawn(PbrBundle {
        mesh: mesh.clone(),
        material,
        transform: object_transform,
        ..default()
    });

    //Portal
    let mut destination_transform = portal_transform.clone();
    destination_transform.translation = Vec3::new(0.,-10.,0.);
    commands.spawn(CreatePortalBundle {
        mesh: mesh,
        create_portal: CreatePortal {
            main_camera: Some(main_camera),
            destination: AsPortalDestination::Create(CreatePortalDestination {
                transform: destination_transform,
            }),
            cull_mode,
            plane_mode: None,
            debug: Some(DebugPortal {
                show_window: false,
                ..default()
            }),
            ..default()
        },
        portal_transform,
        ..default()
    });
}