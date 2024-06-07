//! This example illustrates using portals that are not simple planes
//!
//! On top you can see portals, at the bottom in the center is where their destination is,
//! and around are shapes that you can also see through the portal with the correct orientation

use bevy::{prelude::*, render::render_resource::Face};
use bevy_basic_portals::*;

#[path = "../../helpers/pivot_cameras.rs"]
mod pivot_cameras;
#[path = "../../helpers/textures.rs"]
mod textures;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
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
    // Camera
    let pivot = Vec3::ZERO;
    let main_camera = commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 0., 20.0).looking_at(pivot, Vec3::Y),
                ..default()
            },
            pivot_cameras::PivotCamera {
                pivot,
                closest: 0.,
                ..default()
            },
        ))
        .id();

    // Lights
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.,
    });

    commands.insert_resource(ClearColor(Color::srgb(0., 0., 0.)));

    // Sphere
    let debug_material = materials.add(textures::debug_material(&mut images, 3, Some(Face::Back)));
    let sphere_mesh = meshes.add(Sphere::new(2.5).mesh().uv(32, 18));
    setup_object_and_portal(
        &mut commands,
        main_camera,
        sphere_mesh,
        debug_material.clone(),
        Transform::from_xyz(10., 0., 0.),
        Some(Face::Back),
    );

    // Cube
    let debug_material = materials.add(textures::debug_material(&mut images, 2, Some(Face::Back)));
    let cube_mesh = meshes.add(Cuboid::new(5., 5., 5.));
    setup_object_and_portal(
        &mut commands,
        main_camera,
        cube_mesh,
        debug_material.clone(),
        Transform::from_xyz(-10., 0., 0.),
        Some(Face::Back),
    );

    // double-sided Quad
    let debug_material = materials.add(textures::debug_material(&mut images, 1, None));
    let quad_mesh = meshes.add(Rectangle::new(5., 5.));
    setup_object_and_portal(
        &mut commands,
        main_camera,
        quad_mesh,
        debug_material.clone(),
        Transform::from_xyz(0., 0., -10.),
        None,
    );
}

fn setup_object_and_portal(
    commands: &mut Commands,
    main_camera: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    portal_transform: Transform,
    cull_mode: Option<Face>,
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
    destination_transform.translation = Vec3::new(0., -10., 0.);
    commands.spawn(CreatePortalBundle {
        mesh: mesh,
        create_portal: CreatePortal {
            main_camera: Some(main_camera),
            destination: AsPortalDestination::Create(CreatePortalDestination {
                transform: destination_transform,
                ..default()
            }),
            portal_mode: PortalMode::MaskedImageNoFrustum,
            cull_mode,
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
