use bevy::{
    prelude::*,
    render::view::RenderLayers,
};
use bevy_basic_portals::*;
use std::f32::consts::PI;

pub const DESTINATION_DISTANCE: f32 = 50.;
pub const PORTAL_SIZE: f32 = 10.;

/// Sets up a portal, to be used for one of the cube's face
pub fn setup_portal_cube_face (
    commands: &mut Commands,
    spawn_portal_dir: Vec3,
    spawn_portal_up: Vec3,
    main_camera: Entity,
    render_layer: RenderLayers,
    portal_mesh: Handle<Mesh>,
    automatic: bool
) {
    let mut portal_transform = Transform::from_translation(spawn_portal_dir * (PORTAL_SIZE / 2.));
    portal_transform.look_to(-spawn_portal_dir, spawn_portal_up);

    let destination_transform = get_destination_transform(spawn_portal_dir, spawn_portal_up);

    let create_portal = portals::CreatePortal {
        destination: AsPortalDestination::Create(CreatePortalDestination {
            transform: destination_transform,
            ..default()
        }),
        main_camera: Some(main_camera),
        render_layer,
        ..default()
    };

    // This shows two different ways of creating a portal
    if automatic {
        commands.spawn(CreatePortalBundle {
            mesh: portal_mesh,
            portal_transform,
            create_portal,
            ..default()
        });
    } else {
        commands.spawn(CreatePortalBundle {
            mesh: portal_mesh,
            portal_transform,
            ..default()
        }).add(CreatePortalCommand {
            config: Some(create_portal)
        });
    }
}

/// Sets up the scene at the destination of a portal, to have something interesting to see through the portal
pub fn setup_scene_test (
    commands: &mut Commands,
    spawn_portal_dir: Vec3,
    spawn_portal_up: Vec3,
    render_layer: RenderLayers,
    wall_mesh: Handle<Mesh>,
    wall_material: Handle<StandardMaterial>,
    shape: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    color: Color,
) {
    let destination_transform = get_destination_transform(spawn_portal_dir, spawn_portal_up);

    commands.spawn((
        TransformBundle {
            local: destination_transform,
            ..default()
        },
        VisibilityBundle::default()
    )).with_children(|parent| {
        // Shape
        let mut shape_transform = Transform::default();
        shape_transform.translation.z += - PORTAL_SIZE/2.;
        parent.spawn((
            PbrBundle {
                mesh: shape,
                material,
                transform: shape_transform,
                ..default()
            },
            render_layer
        ));

        // Light
        let mut light_transform = Transform::default();
        light_transform.translation.z += PORTAL_SIZE * 2.;
        parent.spawn((
            PointLightBundle {
                point_light: PointLight {
                    color,
                    intensity: 9_000_000.0,
                    range: DESTINATION_DISTANCE - PORTAL_SIZE,
                    shadows_enabled: true,
                    ..default()
                },
                transform: light_transform,
                ..default()
            },
            render_layer
        ));

        // Walls
        let walls_center_rotation = vec![
            // back
            (Vec3::new(0.,0.,-PORTAL_SIZE), Vec3::Y, 0.),
            // left
            (Vec3::new(-PORTAL_SIZE/2.,0.,-PORTAL_SIZE/2.), Vec3::Y, PI/2.),
            // right
            (Vec3::new(PORTAL_SIZE/2.,0.,-PORTAL_SIZE/2.), Vec3::Y, -PI/2.),
            // up
            (Vec3::new(0.,PORTAL_SIZE/2.,-PORTAL_SIZE/2.), Vec3::X, PI/2.),
            // down
            (Vec3::new(0.,-PORTAL_SIZE/2.,-PORTAL_SIZE/2.), Vec3::X, -PI/2.),
        ];
        for (center, axis, angle) in walls_center_rotation {
            let mut transform = Transform::from_translation(center);
            transform.rotate_axis(axis, angle);
            parent.spawn((
                PbrBundle {
                    mesh: wall_mesh.clone(),
                    transform,
                    material: wall_material.clone(),
                    ..default()
                },
                render_layer
            ));
        }
    });
}

/// Gets a destination transform that is relatively far behind the portal to be able to see both the portal and the destination
/// when using the same RenderLayer while still having lights not interfere with each other (no support for single RenderLayer light in bevy yet)
fn get_destination_transform(
    spawn_portal_dir: Vec3,
    spawn_portal_up: Vec3,
) -> Transform {
    let mut destination_transform = Transform::from_translation(spawn_portal_dir * -DESTINATION_DISTANCE);
    destination_transform.look_to(-spawn_portal_dir, spawn_portal_up);
    destination_transform
}