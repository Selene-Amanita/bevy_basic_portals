///! Components, systems and command for the creation of portals

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_core_pipeline::prelude::*;
use bevy_ecs::{
    prelude::*,
    system::{EntityCommand, SystemState},
};
use bevy_hierarchy::prelude::*;
use bevy_math::prelude::*;
use bevy_pbr::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::{
    prelude::*,
    render_resource::{
        Extent3d,
        Face,
        TextureDescriptor,
        TextureDimension,
        TextureFormat,
        TextureUsages,
    },
    camera::RenderTarget,
};
use bevy_transform::{
    prelude::*,
    TransformSystem,
};
use bevy_window::{
    PrimaryWindow,
    Window,
    WindowLevel,
    WindowResolution,
    WindowRef,
};
use std::f32::consts::PI;

use super::*;

pub(crate) fn build_create(app: &mut App, check_create: &PortalsCheckMode) {
    app
        .register_type::<Portal>()
        .register_type::<PortalDestination>()
        .register_type::<PortalCamera>();

    if check_create != &PortalsCheckMode::Manual {
        app.add_startup_system(create_portals.in_base_set(StartupSet::PostStartup).after(TransformSystem::TransformPropagate));
    }

    if check_create == &PortalsCheckMode::AlwaysCheck {
        app.add_system(create_portals.in_base_set(CoreSet::PostUpdate).after(TransformSystem::TransformPropagate));
    }
}

/// References to the entities that make a portal work
#[derive(Clone, Reflect)]
pub struct PortalParts {
    pub main_camera: Entity,
    pub portal: Entity,
    pub destination: Entity,
    pub portal_camera: Entity,
}

/// Marker component for the portal.
/// 
/// Will replace [CreatePortal] after [create_portals].
#[derive(Component, Reflect)]
pub struct Portal {
    pub parts: PortalParts,
}

/// Marker component for the destination.
/// 
/// Will be added to the entity defined by [CreatePortal.destination](CreatePortal)
#[derive(Component, Reflect)]
pub struct PortalDestination {
    pub parts: PortalParts,
}

/// Component for a portal camera, the camera that is used to see through a portal.
#[derive(Component, Reflect)]
pub struct PortalCamera {
    pub image: Handle<Image>,
    #[reflect(ignore)]
    pub plane_mode: Option<Face>,
    pub parts: PortalParts,
}

/// Marker component for the debug camera when [DebugPortal::show_window] is true.
#[derive(Component)]
pub struct PortalDebugCamera;

/// Command to create a portal manually.
/// 
/// Warning: If [`PortalsPlugin::check_create`](PortalsPlugin) is not [PortalsCheckMode::Manual],
/// and you add this command with a config (not None) to an entity which already has a [CreatePortal] component,
/// this component should be ignored and removed.
/// The only exception is if [`PortalsPlugin::check_create`](PortalsPlugin) is [PortalsCheckMode::AlwaysCheck],
/// the command was added during [CoreSet::PostUpdate], in which case two portal cameras may be created. Don't do that.
#[derive(Default)]
pub struct CreatePortalCommand {
    pub config: Option<CreatePortal>
}

impl EntityCommand for CreatePortalCommand {
    fn write(self, id: Entity, world: &mut World) {
        let (portal_transform, mesh) = world.query::<(&GlobalTransform, &Handle<Mesh>)>().get(world, id)
            .expect("You must provide a GlobalTransform and Handle<Mesh> components to the entity before using a CreatePortalCommand");
        let portal_transform = *portal_transform;
        let mesh = mesh.clone();

        let portal_create = match self.config {
            Some(config) => config,
            None => world.query::<&CreatePortal>().get(world, id)
                .expect("You must provide a CreatePortal component to the entity or to the CreatePortalCommand itself before using it").clone()
        };

        let mut system_state = SystemState::<(
            Commands,
            ResMut<Assets<Image>>,
            ResMut<Assets<PortalMaterial>>,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
            Query<(Entity, &Camera)>,
            Query<&Window, With<PrimaryWindow>>,
            Query<&Window>,
        )>::new(world);
        let (
            mut commands,
            mut images,
            mut portal_materials,
            mut meshes,
            mut materials,
            main_camera_query,
            primary_window_query,
            windows_query
        ) = system_state.get_mut(world);

        create_portal(
            &mut commands,
            &mut images,
            &mut portal_materials,
            &mut meshes,
            &mut materials,
            &main_camera_query,
            &primary_window_query,
            &windows_query,
            id,
            &portal_create,
            &portal_transform,
            &mesh
        );

        system_state.apply(world);
    }
}

/// System that will find entities with the components of [CreatePortalBundle] and create a portal.
/// 
/// It will create a [PortalCamera] at the destination, and put a portal material on the mesh of the entity with [CreatePortal].
/// The [PortalCamera] will render to that material.
/// It will also create debug elements if needed.
/// It will then remove the [CreatePortal] component.
/// 
/// This system can be automatically added by [PortalsPlugin] depending on [PortalsCheckMode].
#[allow(clippy::too_many_arguments)]
pub fn create_portals(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut portal_materials: ResMut<Assets<PortalMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    main_camera_query: Query<(Entity, &Camera)>,
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    windows_query: Query<&Window>,
    portals_to_create: Query<(Entity, &CreatePortal, &GlobalTransform, &Handle<Mesh>)>
) {
    for (portal_entity, portal_create, portal_transform, mesh) in portals_to_create.iter() {
        create_portal(&mut commands, &mut images, &mut portal_materials, &mut meshes, &mut materials,
            &main_camera_query, &primary_window_query, &windows_query,
            portal_entity, portal_create, portal_transform, mesh);
    }
}

/// Creates a portal.
/// 
/// Called from [create_portals] or [CreatePortalCommand].
#[allow(clippy::too_many_arguments)]
fn create_portal(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    portal_materials: &mut ResMut<Assets<PortalMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    main_camera_query: &Query<(Entity, &Camera)>,
    primary_window_query: &Query<&Window, With<PrimaryWindow>>,
    windows_query: &Query<&Window>,
    portal_entity: Entity,
    create_portal: &CreatePortal,
    _portal_global_transform: &GlobalTransform,
    portal_mesh: &Handle<Mesh>
) {
    // Get main camera infos
    let (main_camera_entity, main_camera) = 
        if let Some(camera_entity) = create_portal.main_camera {
            main_camera_query.get(camera_entity).unwrap()
        }
        else {
            main_camera_query.iter().next().unwrap()
        };

    let main_camera_viewport_size = get_viewport_size(main_camera, primary_window_query, windows_query, images);

    let size = Extent3d {
        width: main_camera_viewport_size.x,
        height: main_camera_viewport_size.y,
        ..Extent3d::default()
    };

    // Image that the PortalCamera will render to
    let mut portal_image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..Image::default()
    };

    // Fill portal_image.data with zeroes
    portal_image.resize(size);

    let portal_image = images.add(portal_image);

    // Material that the portal camera will render to
    let portal_material = portal_materials.add(PortalMaterial {
        color_texture: Some(portal_image.clone()),
        cull_mode: create_portal.cull_mode
    });

    // Create or get the destination entity
    let destination_entity = match create_portal.destination {
        AsPortalDestination::Use(entity) => entity,
        AsPortalDestination::Create(CreatePortalDestination{transform, parent}) => {
            let mut destination_commands = commands.spawn(SpatialBundle{
                transform,
                global_transform: GlobalTransform::from(transform),
                ..SpatialBundle::default()
            });
            if let Some(parent) = parent {
                destination_commands.set_parent(parent);
            }
            destination_commands.id()
        }
        AsPortalDestination::CreateMirror => {
            let mut destination_commands = commands.spawn(SpatialBundle{
                transform: Transform::from_rotation(Quat::from_axis_angle(Vec3::Y, PI)),
                ..SpatialBundle::default()
            });
            destination_commands.set_parent(portal_entity);
            destination_commands.id()
        },
    };

    // Create the portal camera
    let portal_camera_entity = commands.spawn((
        Camera3dBundle {
            camera: Camera {
                order: -1,
                target: RenderTarget::Image(portal_image.clone()),
                ..Camera::default()
            },
            // TOFIX set the exact value of Transform and GlobalTransform to avoid black screen at spawn
            // let portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, &destination_transform);
            // This requires an extra Query to get destination_transform when AsPortalDestination::Entity/CreateMirror
            // Would still matter if the portal camera is a child of the destination
            //transform: portal_camera_transform,
            //global_transorm: GlobalTransform::from(portal_camera_transform),
            ..Camera3dBundle::default()
        },
        VisibilityBundle {
            visibility: Visibility::Hidden,
            ..VisibilityBundle::default()
        },
        create_portal.render_layer
    )).id();

    // Add portal components
    let parts = PortalParts {
        main_camera: main_camera_entity,
        portal: portal_entity,
        destination: destination_entity,
        portal_camera: portal_camera_entity,
    };

    let mut portal_entity_command = commands.entity(portal_entity);
    portal_entity_command.insert(portal_material);
    portal_entity_command.remove::<CreatePortal>();
    portal_entity_command.insert(Portal {
        parts: parts.clone(),
    });

    commands.entity(portal_camera_entity).insert(PortalCamera {
        image: portal_image,
        plane_mode: create_portal.plane_mode,
        parts: parts.clone(),
    });

    commands.entity(destination_entity).insert(PortalDestination{
        parts,
    });

    // Debug
    if let Some(debug) = &create_portal.debug {
        let debug_color = debug.color;
        let mut debug_transparent_color = debug.color;
        debug_transparent_color.set_a(0.3);

        // Create the debug camera as a child of the portal camera in a new window
        if debug.show_window {
            let debug_window = commands.spawn(Window {
                title: (match &debug.name {Some(name) => name, _ => "Debug"}).to_owned(),
                resolution: WindowResolution::new(size.width as f32, size.height as f32),
                window_level: WindowLevel::AlwaysOnBottom,
                ..Window::default()
            }).id();
            commands.entity(portal_camera_entity).with_children(|parent| {
                parent.spawn((
                    Camera3dBundle {
                        camera: Camera {
                            order: -1,
                            target: RenderTarget::Window(WindowRef::Entity(debug_window)),
                            ..Camera::default()
                        },
                        ..Camera3dBundle::default()
                    },
                    PortalDebugCamera {
                    },
                    create_portal.render_layer
                ));
            });
        }

        // Put a sphere at destination_transform.translation, as a child of the destination
        if debug.show_destination_point {
            commands.entity(destination_entity).with_children(|parent| {
                parent.spawn((
                    PbrBundle {
                        mesh: meshes.add(shape::Icosphere {radius:0.1, ..shape::Icosphere::default()}.try_into().unwrap()),
                        material: materials.add(debug_color.into()),
                        ..PbrBundle::default()
                    },
                    create_portal.render_layer
                ));
            });
        }

        // Put a semi-transparent double-sided copy of the portal mesh at destination_transform, asa child of the destination
        if debug.show_portal_copy {
            let mut portal_copy_material: StandardMaterial = debug_transparent_color.into();
            portal_copy_material.cull_mode = create_portal.cull_mode;
            commands.entity(destination_entity).with_children(|parent| {
                parent.spawn((
                    PbrBundle {
                        mesh: portal_mesh.clone(),
                        material: materials.add(portal_copy_material),
                        ..PbrBundle::default()
                    },
                    create_portal.render_layer
                ));
            });
        }

        // Put a sphere at the portal camera position, as a child of the portal camera
        if debug.show_portal_camera_point {
            commands.entity(portal_camera_entity).with_children(|parent| {
                parent.spawn((
                    PbrBundle {
                        mesh: meshes.add(shape::Icosphere {radius:0.1, ..shape::Icosphere::default()}.try_into().unwrap()),
                        material: materials.add(debug_color.into()),
                        visibility: Visibility::Visible,
                        ..PbrBundle::default()
                    },
                    create_portal.render_layer
                ));
            });
        }
    }
}