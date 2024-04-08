///! Components, systems and command for the creation of portals

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_core_pipeline::{
    prelude::*,
    tonemapping::{Tonemapping, DebandDither}
};
use bevy_ecs::{
    prelude::*,
    system::{EntityCommand, SystemState, SystemParam},
};
use bevy_hierarchy::prelude::*;
use bevy_math::prelude::*;
use bevy_pbr::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::{
    prelude::*,
    render_resource::{
        Extent3d,
        TextureDescriptor,
        TextureDimension,
        TextureFormat,
        TextureUsages,
    },
    camera::RenderTarget,
    view::ColorGrading,
};
use bevy_transform::{
    prelude::*,
    TransformSystem,
};
use bevy_window::{
    Window,
    WindowLevel,
    WindowResolution,
    WindowRef,
};
use std::f32::consts::PI;

use super::*;

/// Add the create logic to [PortalsPlugin]
pub(super) fn build_create(app: &mut App, check_create: &PortalsCheckMode) {
    app
        .register_type::<Portal>()
        .register_type::<PortalDestination>()
        .register_type::<PortalCamera>();

    if check_create != &PortalsCheckMode::Manual {
        app.add_systems(PostStartup, create_portals.after(TransformSystem::TransformPropagate));
    }

    if check_create == &PortalsCheckMode::AlwaysCheck {
        app.add_systems(PostUpdate, create_portals.after(TransformSystem::TransformPropagate));
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

/// Marker [Component] for the portal.
/// 
/// Will replace [CreatePortal] after [create_portals].
#[derive(Component, Reflect)]
pub struct Portal {
    pub parts: PortalParts,
}

/// Marker [Component] for the destination.
/// 
/// Will be added to the entity defined by [CreatePortal.destination](CreatePortal)
#[derive(Component, Reflect)]
pub struct PortalDestination {
    pub parts: PortalParts,
}

/// [Component] for a portal camera, the camera that is used to see through a portal.
/// 
/// Note: The entity this component is attached to is not supposed to be a child of another entity.
#[derive(Component, Reflect)]
pub struct PortalCamera {
    pub image: Handle<Image>,
    #[reflect(ignore)]
    pub portal_mode: PortalMode,
    pub parts: PortalParts,
}

/// Marker [Component] for the debug camera when [DebugPortal::show_window] is true.
#[derive(Component)]
pub struct PortalDebugCamera;

/// [EntityCommand] to create a portal manually.
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
    fn apply(self, id: Entity, world: &mut World) {
        let (portal_transform, mesh) = world.query::<(&GlobalTransform, &Handle<Mesh>)>().get(world, id)
            .expect("You must provide a GlobalTransform and Handle<Mesh> components to the entity before using a CreatePortalCommand");
        let portal_transform = *portal_transform;
        let mesh = mesh.clone();

        let portal_create = match self.config {
            Some(config) => config,
            None => world.query::<&CreatePortal>().get(world, id)
                .expect("You must provide a CreatePortal component to the entity or to the CreatePortalCommand itself before using it").clone()
        };

        let mut system_state = SystemState::<CreatePortalParams>::new(world);
        let mut create_params = system_state.get_mut(world);

        create_portal(
            &mut create_params,
            id,
            &portal_create,
            &portal_transform,
            &mesh
        );

        system_state.apply(world);
    }
}

/// [System] that will find entities with the components of [CreatePortalBundle] and create a portal.
/// 
/// It will create a [PortalCamera] at the destination, and put a portal material on the mesh of the entity with [CreatePortal].
/// The [PortalCamera] will render to that material.
/// It will also create debug elements if needed.
/// It will then remove the [CreatePortal] component.
/// 
/// This system can be automatically added by [PortalsPlugin] depending on [PortalsCheckMode].
#[allow(clippy::too_many_arguments)]
pub fn create_portals(
    mut create_params: CreatePortalParams,
    portals_to_create: Query<(Entity, &CreatePortal, &GlobalTransform, &Handle<Mesh>)>
) {
    for (portal_entity, portal_create, portal_transform, mesh) in portals_to_create.iter() {
        create_portal(&mut create_params, portal_entity, portal_create, portal_transform, mesh);
    }
}

/// Creates a portal.
/// 
/// Called from [create_portals] or [CreatePortalCommand].
#[allow(clippy::too_many_arguments)]
fn create_portal(
    CreatePortalParams {
        commands,
        portal_materials,
        meshes,
        materials,
        main_camera_query,
        size_params
    }: &mut CreatePortalParams,
    portal_entity: Entity,
    create_portal: &CreatePortal,
    _portal_global_transform: &GlobalTransform,
    portal_mesh: &Handle<Mesh>
) {
    // Get main camera infos
    let (
        main_camera_entity,
        main_camera,
        main_camera_projection,
        main_camera_camera3d,
        main_camera_tonemapping,
        main_camera_dither,
        main_camera_color_grading,
    ) = 
        if let Some(camera_entity) = create_portal.main_camera {
            main_camera_query.get(camera_entity).unwrap()
        }
        else {
            main_camera_query.iter().next().unwrap()
        };

    let main_camera_viewport_size = get_viewport_size(main_camera, size_params);

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

    let portal_image = size_params.images.add(portal_image);

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
    let camera_bundle = Camera3dBundle::default();
    let projection: PortalProjection = main_camera_projection
        .unwrap_or(&camera_bundle.projection)
        .clone()
        .into();
    let portal_camera_entity = commands.spawn((
        Camera {
            order: -1,
            target: RenderTarget::Image(portal_image.clone()),
            ..Camera::default()
        },
        projection,
        camera_bundle.camera_render_graph,
        camera_bundle.visible_entities,
        camera_bundle.frustum,
        main_camera_camera3d.unwrap_or(&camera_bundle.camera_3d).clone(),
        *main_camera_tonemapping.unwrap_or(&camera_bundle.tonemapping),
        *main_camera_dither.unwrap_or(&camera_bundle.dither),
        *main_camera_color_grading.unwrap_or(&camera_bundle.color_grading),
        // TOFIX set the exact value of Transform and GlobalTransform to avoid black screen at spawn
        // let portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, &destination_transform);
        // This requires an extra Query to get destination_transform when AsPortalDestination::Entity/CreateMirror
        // Would still matter if the portal camera is a child of the destination
        //transform: portal_camera_transform,
        //global_transorm: GlobalTransform::from(portal_camera_transform),
        SpatialBundle {
            visibility: Visibility::Hidden,
            ..SpatialBundle::default()
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
        portal_mode: create_portal.portal_mode.clone(),
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
                        mesh: meshes.add(Sphere::new(0.1).mesh().ico(5).unwrap()),
                        material: materials.add(debug_color),
                        ..PbrBundle::default()
                    },
                    create_portal.render_layer
                ));
            });
        }

        // Put a semi-transparent double-sided copy of the portal mesh at destination_transform,
        // as a child of the destination.
        if debug.show_portal_copy {
            let mut portal_copy_material: StandardMaterial = debug_transparent_color.into();
            portal_copy_material.cull_mode = create_portal.cull_mode;
            commands.entity(destination_entity).with_children(|parent| {
                parent.spawn((
                    PbrBundle {
                        mesh: portal_mesh.clone(),
                        material: materials.add(portal_copy_material),
                        // So that it can still be seen through the portal,
                        // despite rounding frustum mismatch
                        transform: Transform::from_xyz(0., 0., -0.001),
                        ..PbrBundle::default()
                    },
                    create_portal.render_layer
                ));
            });
        }

        // Put a sphere at the portal camera position, as a child of the portal camera.
        if debug.show_portal_camera_point {
            commands.entity(portal_camera_entity).with_children(|parent| {
                parent.spawn((
                    PbrBundle {
                        mesh: meshes.add(Sphere::new(0.1).mesh().ico(5).unwrap()),
                        material: materials.add(debug_color),
                        visibility: Visibility::Visible,
                        ..PbrBundle::default()
                    },
                    create_portal.render_layer
                ));
            });
        }
    }
}

/// [SystemParam] needed for [create_portals]
#[derive(SystemParam)]
pub struct CreatePortalParams<'w, 's> {
    commands: Commands<'w, 's>,
    portal_materials: ResMut<'w, Assets<PortalMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    main_camera_query: Query<'w, 's, (
        Entity,
        &'static Camera,
        Option<&'static Projection>,
        Option<&'static Camera3d>,
        Option<&'static Tonemapping>,
        Option<&'static DebandDither>,
        Option<&'static ColorGrading>,
    )>,
    size_params: PortalImageSizeParams<'w, 's>,
}