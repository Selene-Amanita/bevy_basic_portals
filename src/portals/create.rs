//! Components, systems and command for the creation of portals

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_camera::{Exposure, RenderTarget, prelude::*};
use bevy_color::Alpha;
use bevy_core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy_ecs::{
    prelude::*,
    system::{SystemParam, SystemState},
};
use bevy_image::Image;
#[cfg(feature = "debug_no_shadow")]
use bevy_light::NotShadowCaster;
use bevy_math::prelude::*;
use bevy_mesh::prelude::*;
use bevy_pbr::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::{
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    view::ColorGrading,
};
use bevy_transform::prelude::*;
use bevy_window::{Window, WindowRef, WindowResolution};
use std::f32::consts::PI;
use tracing::error;

use super::*;

/// Add the create logic to [PortalsPlugin]
pub(super) fn build_create(app: &mut App) {
    app.register_type::<Portal>()
        .register_type::<PortalDestination>()
        .register_type::<PortalCamera>();

    app.add_observer(create_portal_on_add);
}

/// [Component] referencing the entities that make a portal work.
///
/// Will be put on a separate entity.
#[derive(Component, Reflect)]
pub struct PortalParts {
    pub main_camera: Entity,
    pub portal: Entity,
    pub destination: Entity,
    pub portal_camera: Entity,
}

/// [Component] put on any portal part (except the main camera) to reference the entity referencing the other parts.
#[derive(Component, Reflect)]
pub struct PortalPart {
    pub parts: Entity,
}

/// Marker [Component] for the portal.
///
/// Will replace [CreatePortal] after [create_portals].
#[derive(Component, Reflect)]
pub struct Portal;

/// Marker [Component] for the destination.
///
/// Will be added to the entity defined by [CreatePortal.destination](CreatePortal)
#[derive(Component, Reflect, Default)]
pub struct PortalDestination {
    /// Mirrors the image with origin and normal, see [MirrorConfig]
    pub mirror: Option<(Vec3, Dir3)>,
}

/// [Component] for a portal camera, the camera that is used to see through a portal.
///
/// Note: The entity this component is attached to is not supposed to be a child of another entity.
#[derive(Component, Reflect)]
pub struct PortalCamera {
    pub image: Handle<Image>,
    #[reflect(ignore)]
    pub portal_mode: PortalMode,
}

/// Marker [Component] for the debug camera when [DebugPortal::show_portal_texture] is not None.
#[derive(Component)]
pub struct PortalDebugCamera;

/// Marker [Component] for the debug UI root [`bevy_ui::Node`] when [DebugPortal::show_portal_texture] is DebugPortalTextureView.
#[derive(Component)]
pub struct PortalDebugUIRootNode;

/// [EntityCommand] to create a portal manually.
///
/// Warning: If [`PortalsPlugin::check_create`](PortalsPlugin) is not [PortalsCheckMode::Manual],
/// and you add this command with a config (not None) to an entity which already has a [CreatePortal] component,
/// this component should be ignored and removed.
/// The only exception is if [`PortalsPlugin::check_create`](PortalsPlugin) is [PortalsCheckMode::AlwaysCheck],
/// the command was added during [CoreSet::PostUpdate], in which case two portal cameras may be created. Don't do that.
#[derive(Default)]
pub struct CreatePortalCommand {
    pub config: Option<CreatePortal>,
}

impl EntityCommand for CreatePortalCommand {
    fn apply(self, mut entity_world: EntityWorldMut) {
        let id = entity_world.id();
        entity_world.world_scope(move |world: &mut World| {
            let (portal_transform, mesh) = world.query::<(&Transform, &Mesh3d)>().get(world, id)
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
                &mesh,
            );

            system_state.apply(world);
        });
    }
}

/// [Observer] triggering on adding [CreatePortal], that will remove this component and create a real portal.
///
/// It will create a [PortalCamera] at the destination, and put a portal material on the mesh of the entity with [CreatePortal].
/// The [PortalCamera] will render to that material.
/// It will also create debug elements if needed.
/// It will then remove the [CreatePortal] component.
pub fn create_portal_on_add(
    trigger: On<Add, CreatePortal>,
    mut create_params: CreatePortalParams,
    portal_query: Query<(&CreatePortal, &Transform, &Mesh3d)>, //TODO revert !dbg()
) {
    let portal_entity = trigger.event().entity;
    let Ok((portal_create, portal_transform, mesh)) = portal_query.get(portal_entity) else {
        error!("Entity with CreatePortal lacks the other required components");
        return;
    };

    create_portal(
        &mut create_params,
        portal_entity,
        portal_create,
        portal_transform,
        mesh,
    );
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
        #[cfg(feature = "debug_ui")]
        debug_ui_root_query,
        size_params,
    }: &mut CreatePortalParams,
    portal_entity: Entity,
    create_portal: &CreatePortal,
    _portal_global_transform: &Transform, //TODO revert !dbg()
    portal_mesh: &Handle<Mesh>,
) {
    // Get main camera infos
    let (
        main_camera_entity,
        main_camera,
        main_camera_target,
        main_camera_projection,
        main_camera_camera3d,
        main_camera_tonemapping,
        main_camera_deband_dither,
        main_camera_color_grading,
        main_camera_exposure,
    ) = if let Some(camera_entity) = create_portal.main_camera {
        main_camera_query.get(camera_entity).unwrap()
    } else {
        main_camera_query.iter().next().unwrap()
    };

    let main_camera_viewport_size = get_viewport_size(main_camera, main_camera_target, size_params)
        .unwrap_or_else(|| {
            error!("Viewport size not found, creating portal with default sized image");
            UVec2::new(100, 100)
        });

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

    // Create or get the destination entity
    let (destination_entity, mirror_u, mirror_v) = match create_portal.destination {
        PortalDestinationSource::Use(entity) => {
            commands.entity(entity).insert(PortalDestination::default());
            (entity, false, false)
        }
        PortalDestinationSource::Create(CreatePortalDestination {
            transform,
            parent,
            ref mirror,
        }) => {
            let (mirror, mirror_u, mirror_v) = if let Some(MirrorConfig {
                origin,
                normal,
                mirror_u,
                mirror_v,
            }) = mirror
            {
                (Some((*origin, *normal)), *mirror_u, *mirror_v)
            } else {
                (None, false, false)
            };
            let mut destination_commands = commands.spawn((
                transform,
                GlobalTransform::from(transform),
                PortalDestination { mirror },
            ));
            if let Some(parent) = parent {
                destination_commands.insert(ChildOf(parent));
            }
            (destination_commands.id(), mirror_u, mirror_v)
        }
        PortalDestinationSource::CreateMirror => {
            let destination_commands = commands.spawn((
                Transform::from_rotation(Quat::from_axis_angle(Vec3::Y, PI)),
                PortalDestination {
                    mirror: Some((Vec3::ZERO, Dir3::X)),
                },
                ChildOf(portal_entity),
            ));
            (destination_commands.id(), true, false)
        }
    };

    // Material that the portal camera will render to
    let portal_material = portal_materials.add(PortalMaterial {
        color_texture: Some(portal_image.clone()),
        cull_mode: create_portal.cull_mode,
        mirror_u: if mirror_u { 1 } else { 0 },
        mirror_v: if mirror_v { 1 } else { 0 },
    });

    // Create the portal camera
    let portal_camera_entity = commands
        .spawn((
            main_camera_camera3d
                .cloned()
                .unwrap_or_else(Camera3d::default),
            Camera {
                order: -1,
                ..Camera::default()
            },
            RenderTarget::Image(portal_image.clone().into()),
            main_camera_projection
                .cloned()
                .unwrap_or_else(Projection::default),
            main_camera_tonemapping
                .cloned()
                .unwrap_or_else(Tonemapping::default),
            main_camera_deband_dither
                .cloned()
                .unwrap_or_else(DebandDither::default),
            main_camera_color_grading
                .cloned()
                .unwrap_or_else(ColorGrading::default),
            main_camera_exposure
                .cloned()
                .unwrap_or_else(Exposure::default),
            Visibility::Hidden,
            create_portal.render_layer.clone(),
            PortalCamera {
                image: portal_image,
                portal_mode: create_portal.portal_mode.clone(),
            },
            // TOFIX set the exact value of Transform and GlobalTransform to avoid black screen at spawn
            // let portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, &destination_transform);
            // This requires an extra Query to get destination_transform when AsPortalDestination::Entity/CreateMirror
            // Would still matter if the portal camera is a child of the destination
            //transform: portal_camera_transform,
            //global_transorm: GlobalTransform::from(portal_camera_transform),
        ))
        .id();

    // Add portal components
    let parts = commands
        .spawn(PortalParts {
            main_camera: main_camera_entity,
            portal: portal_entity,
            destination: destination_entity,
            portal_camera: portal_camera_entity,
        })
        .id();

    let mut portal_entity_command = commands.entity(portal_entity);
    portal_entity_command.remove::<CreatePortal>();
    portal_entity_command.insert((
        Portal,
        PortalPart { parts },
        MeshMaterial3d(portal_material),
    ));

    commands
        .entity(portal_camera_entity)
        .insert((PortalPart { parts },));

    commands
        .entity(destination_entity)
        .insert(PortalPart { parts });

    // Debug
    if let Some(debug) = &create_portal.debug {
        let debug_color = debug.color;
        let mut debug_transparent_color = debug.color;
        debug_transparent_color.set_alpha(0.3);

        // Show a debug view of the portal camera
        match debug.show_portal_texture {
            DebugPortalTextureView::Window => {
                // Create the debug camera as a child of the portal camera in a new window
                let debug_window = commands
                    .spawn(Window {
                        title: (match &debug.name {
                            Some(name) => name,
                            _ => "Portal camera debug",
                        })
                        .to_owned(),
                        resolution: WindowResolution::new(size.width, size.height),
                        ..Window::default()
                    })
                    .id();
                commands.entity(portal_camera_entity).with_child((
                    Camera3d::default(),
                    Camera {
                        order: -1,
                        ..Camera::default()
                    },
                    RenderTarget::Window(WindowRef::Entity(debug_window)),
                    PortalDebugCamera {},
                    create_portal.render_layer.clone(),
                ));
            }
            #[cfg(feature = "debug_ui")]
            DebugPortalTextureView::Widget(ratio) => {
                // Same but in a UI Node
                use bevy_asset::RenderAssetUsages;
                use bevy_color::Color;
                use bevy_ecs::query::QuerySingleError;
                use bevy_ui::prelude::*;

                if let Some(root) = match debug_ui_root_query.single() {
                    Ok(root) => Some(root),
                    Err(QuerySingleError::NoEntities(_)) => Some(
                        commands
                            .spawn((
                                PortalDebugUIRootNode,
                                Node {
                                    height: Val::Percent(100.),
                                    width: Val::Percent(100.),
                                    flex_wrap: FlexWrap::Wrap,
                                    flex_direction: FlexDirection::Column,
                                    align_content: AlignContent::SpaceBetween,
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..Node::default()
                                },
                            ))
                            .id(),
                    ),
                    Err(QuerySingleError::MultipleEntities(_)) => {
                        error!("Multiple Portal Debug UI Root entities.");
                        None
                    }
                } {
                    let mut image = Image::new_uninit(
                        Extent3d::default(),
                        TextureDimension::D2,
                        TextureFormat::Bgra8UnormSrgb,
                        RenderAssetUsages::all(),
                    );
                    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_DST
                        | TextureUsages::RENDER_ATTACHMENT;
                    let image_handle = size_params.images.add(image);
                    let camera = commands
                        .spawn((
                            Camera3d::default(),
                            Camera {
                                order: -1,
                                ..Camera::default()
                            },
                            RenderTarget::Image(image_handle.clone().into()),
                            PortalDebugCamera {},
                            create_portal.render_layer.clone(),
                        ))
                        .id();
                    commands.entity(portal_camera_entity).add_child(camera);

                    commands.entity(root).with_child((
                        Node {
                            position_type: PositionType::Relative,
                            width: Val::Percent(ratio * 100.),
                            height: Val::Percent(ratio * 100.),
                            border: UiRect::all(Val::Px(2.)),
                            ..Node::default()
                        },
                        BorderColor::all(Color::WHITE),
                        ViewportNode::new(camera),
                        UiTargetCamera(main_camera_entity),
                    ));
                }
            }
            DebugPortalTextureView::None => {}
        }

        // Put a sphere at destination_transform.translation, as a child of the destination
        if debug.show_destination_point {
            commands
                .entity(destination_entity)
                .insert(Visibility::default())
                .with_child((
                    Mesh3d(meshes.add(Sphere::new(0.1))),
                    MeshMaterial3d(materials.add(debug_color)),
                    create_portal.render_layer.clone(),
                    #[cfg(feature = "debug_no_shadow")]
                    NotShadowCaster,
                ));
        }

        // Put a semi-transparent double-sided copy of the portal mesh at destination_transform,
        // as a child of the destination.
        if debug.show_portal_copy {
            let mut portal_copy_material: StandardMaterial = debug_transparent_color.into();
            portal_copy_material.cull_mode = create_portal.cull_mode;
            commands.entity(destination_entity).with_child((
                Mesh3d(portal_mesh.clone()),
                MeshMaterial3d(materials.add(portal_copy_material)),
                // So that it can still be seen through the portal,
                // despite rounding frustum mismatch
                Transform::from_xyz(0., 0., -0.001),
                create_portal.render_layer.clone(),
                #[cfg(feature = "debug_no_shadow")]
                NotShadowCaster,
            ));
        }

        // Put a sphere at the portal camera position, as a child of the portal camera.
        if debug.show_portal_camera_point {
            commands.entity(portal_camera_entity).with_child((
                Mesh3d(meshes.add(Sphere::new(0.1))),
                MeshMaterial3d(materials.add(debug_color)),
                Visibility::Visible,
                create_portal.render_layer.clone(),
                #[cfg(feature = "debug_no_shadow")]
                NotShadowCaster,
            ));
        }
    }
}

/// [SystemParam] needed for [create_portals]
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub struct CreatePortalParams<'w, 's> {
    commands: Commands<'w, 's>,
    portal_materials: ResMut<'w, Assets<PortalMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    main_camera_query: Query<
        'w,
        's,
        (
            Entity,
            &'static Camera,
            &'static RenderTarget,
            Option<&'static Projection>,
            Option<&'static Camera3d>,
            Option<&'static Tonemapping>,
            Option<&'static DebandDither>,
            Option<&'static ColorGrading>,
            Option<&'static Exposure>,
        ),
    >,
    #[cfg(feature = "debug_ui")]
    debug_ui_root_query: Query<'w, 's, Entity, With<PortalDebugUIRootNode>>,
    size_params: PortalImageSizeParams<'w, 's>,
}
