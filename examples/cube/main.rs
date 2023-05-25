//! This example creates a cube with each face being a portal to a different scene, using [RenderLayers]
//!
//! (This is what this crate was created for originally)

use bevy::{
    prelude::*,
    render::{render_resource::Face, view::RenderLayers},
};

use bevy_basic_portals::*;
use helpers::{pivot_cameras, textures};

pub mod scenes;

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
    // Main Camera
    let pivot = Vec3::ZERO;
    let main_camera = commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 0., 20.0).looking_at(pivot, Vec3::Y),
                ..default()
            },
            pivot_cameras::PivotCamera {
                pivot,
                closest: 10., // half diagonal of the cube = sqrt(3) * 10 / 2 < 10.
            },
        ))
        .id();

    // Lights
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.01,
    });

    commands.insert_resource(ClearColor(Color::rgb(0., 0., 0.)));

    // Scenes
    let portal_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        scenes::PORTAL_SIZE,
        scenes::PORTAL_SIZE,
    ))));

    let debug_material = materials.add(textures::debug_material(&mut images, 1, Some(Face::Back)));

    let wall_material = materials.add(textures::debug_material(&mut images, 2, Some(Face::Back)));

    // Front scene
    let spawn_portal_dir = Vec3::Z;
    let spawn_portal_up = Vec3::Y;
    let render_layer = RenderLayers::layer(1);
    let shape = meshes.add(shape::Cube::new(5.).into());
    let color = Color::YELLOW;
    scenes::setup_portal_cube_face(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        main_camera,
        render_layer,
        portal_mesh.clone(),
        true,
    );
    scenes::setup_scene_test(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        render_layer,
        portal_mesh.clone(),
        wall_material.clone(),
        shape.clone(),
        debug_material.clone(),
        color,
    );

    // Back scene
    let spawn_portal_dir = -Vec3::Z;
    let spawn_portal_up = Vec3::Y;
    let render_layer = RenderLayers::layer(2);
    let shape = meshes
        .add(shape::Box::from_corners(Vec3::new(1., 4., 1.), Vec3::new(-1., -1., -2.)).into());
    let color = Color::BLUE;
    scenes::setup_portal_cube_face(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        main_camera,
        render_layer,
        portal_mesh.clone(),
        true,
    );
    scenes::setup_scene_test(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        render_layer,
        portal_mesh.clone(),
        wall_material.clone(),
        shape.clone(),
        debug_material.clone(),
        color,
    );

    // Right scene
    let spawn_portal_dir = Vec3::X;
    let spawn_portal_up = Vec3::Y;
    let render_layer = RenderLayers::layer(3);
    let shape = meshes.add(
        shape::Capsule {
            radius: 3.,
            depth: 3.,
            ..default()
        }
        .into(),
    );
    let color = Color::GREEN;
    scenes::setup_portal_cube_face(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        main_camera,
        render_layer,
        portal_mesh.clone(),
        true,
    );
    scenes::setup_scene_test(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        render_layer,
        portal_mesh.clone(),
        wall_material.clone(),
        shape.clone(),
        debug_material.clone(),
        color,
    );

    // Left scene
    let spawn_portal_dir = -Vec3::X;
    let spawn_portal_up = Vec3::Y;
    let render_layer = RenderLayers::layer(4);
    let shape = meshes.add(
        shape::Capsule {
            radius: 3.,
            depth: 3.,
            ..default()
        }
        .into(),
    );
    let color = Color::FUCHSIA;
    scenes::setup_portal_cube_face(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        main_camera,
        render_layer,
        portal_mesh.clone(),
        false,
    );
    scenes::setup_scene_test(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        render_layer,
        portal_mesh.clone(),
        wall_material.clone(),
        shape.clone(),
        debug_material.clone(),
        color,
    );

    // Up scene
    let spawn_portal_dir = Vec3::Y;
    let spawn_portal_up = -Vec3::Z;
    let render_layer = RenderLayers::layer(5);
    let shape = meshes.add(shape::Cube::new(5.).into());
    let color = Color::RED;
    scenes::setup_portal_cube_face(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        main_camera,
        render_layer,
        portal_mesh.clone(),
        false,
    );
    scenes::setup_scene_test(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        render_layer,
        portal_mesh.clone(),
        wall_material.clone(),
        shape.clone(),
        debug_material.clone(),
        color,
    );

    // Down scene
    let spawn_portal_dir = -Vec3::Y;
    let spawn_portal_up = -Vec3::Z;
    let render_layer = RenderLayers::layer(6);
    let shape = meshes.add(
        shape::Capsule {
            radius: 3.,
            depth: 3.,
            ..default()
        }
        .into(),
    );
    let color = Color::CYAN;
    scenes::setup_portal_cube_face(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        main_camera,
        render_layer,
        portal_mesh.clone(),
        false,
    );
    scenes::setup_scene_test(
        &mut commands,
        spawn_portal_dir,
        spawn_portal_up,
        render_layer,
        portal_mesh.clone(),
        wall_material.clone(),
        shape.clone(),
        debug_material.clone(),
        color,
    );
}
