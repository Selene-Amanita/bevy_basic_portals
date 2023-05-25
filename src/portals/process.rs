///! Components, systems and others for the implementation of portals, or doing something specific with them like manual creation

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        camera::RenderTarget,
    },
    window::*,
    ecs::{
        system::{EntityCommand, SystemState},
        query::QueryEntityError
    },
    transform::TransformSystem
};
use std::f32::consts::PI;

use super::*;

const PLANE_MODE_TRIGGER: f32 = 0.2;

pub(super) struct PortalsProcessPlugin {
    pub config: PortalsPlugin
}

impl Plugin for PortalsProcessPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(self.config.despawn_strategy)
            .register_type::<Portal>()
            .register_type::<PortalDestination>()
            .register_type::<PortalCamera>();

        app.add_system(update_portal_cameras.in_base_set(CoreSet::Last));

        if self.config.check_create != PortalsCheckMode::Manual {
            app.add_startup_system(create_portals.in_base_set(StartupSet::PostStartup).after(TransformSystem::TransformPropagate));
        }

        if self.config.check_create == PortalsCheckMode::AlwaysCheck {
            app.add_system(create_portals.in_base_set(CoreSet::PostUpdate).after(TransformSystem::TransformPropagate));
        }
        
        if self.config.check_portal_camera_despawn {
            app.add_system(check_portal_camera_despawn);
        }
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
        ..default()
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
        ..default()
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
                ..default()
            });
            if let Some(parent) = parent {
                destination_commands.set_parent(parent);
            }
            destination_commands.id()
        }
        AsPortalDestination::CreateMirror => {
            let mut destination_commands = commands.spawn(SpatialBundle{
                transform: Transform::from_rotation(Quat::from_axis_angle(Vec3::Y, PI)),
                ..default()
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
                ..default()
            },
            // TOFIX set the exact value of Transform and GlobalTransform to avoid black screen at spawn
            // let portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, &destination_transform);
            // This requires an extra Query to get destination_transform when AsPortalDestination::Entity/CreateMirror
            // Would still matter if the portal camera is a child of the destination
            //transform: portal_camera_transform,
            //global_transorm: GlobalTransform::from(portal_camera_transform),
            ..default()
        },
        VisibilityBundle {
            visibility: Visibility::Hidden,
            ..default()
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
                ..default()
            }).id();
            commands.entity(portal_camera_entity).with_children(|parent| {
                parent.spawn((
                    Camera3dBundle {
                        camera: Camera {
                            order: -1,
                            target: RenderTarget::Window(WindowRef::Entity(debug_window)),
                            ..default()
                        },
                        ..default()
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
                        mesh: meshes.add(shape::Icosphere {radius:0.1, ..default()}.try_into().unwrap()),
                        material: materials.add(debug_color.into()),
                        ..default()
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
                        ..default()
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
                        mesh: meshes.add(shape::Icosphere {radius:0.1, ..default()}.try_into().unwrap()),
                        material: materials.add(debug_color.into()),
                        visibility: Visibility::Visible,
                        ..default()
                    },
                    create_portal.render_layer
                ));
            });
        }
    }
}

/// Moves the [PortalCamera] to follow the main camera relative to the portal and the destination.
#[allow(clippy::too_many_arguments)]
pub fn update_portal_cameras(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    strategy: Res<PortalPartsDespawnStrategy>,
    mut portal_cameras: Query<(&PortalCamera, &mut Transform, &mut GlobalTransform, &mut Camera)>,
    main_camera_query: Query<(&GlobalTransform, &Camera), Without<PortalCamera>>,
    portal_query: Query<&GlobalTransform,(With<Portal>, Without<Camera>)>,
    destination_query: Query<&GlobalTransform, (With<PortalDestination>, Without<Camera>)>,
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    windows_query: Query<&Window>,
) {
    // For every portal camera
    for (portal_camera, mut portal_camera_transform, mut portal_camera_global_transform, mut camera)
        in portal_cameras.iter_mut() {

        // Main Camera
        let main_camera_result = main_camera_query.get(portal_camera.parts.main_camera);
        if let Err(query_error) = main_camera_result {
            deal_with_part_query_error(&mut commands, &portal_camera.parts, &strategy, &query_error, "Main Camera");
            return;
        }
        let (main_camera_transform, main_camera) = main_camera_result.unwrap();
        let main_camera_transform = &main_camera_transform.compute_transform();

        // Portal
        let portal_result = portal_query.get(portal_camera.parts.portal);
        if let Err(query_error) = portal_result {
            deal_with_part_query_error(&mut commands, &portal_camera.parts, &strategy, &query_error, "Portal");
            return;
        }
        let portal_transform = portal_result.unwrap();
        let portal_transform = &portal_transform.compute_transform();

        // Check if portal camera should update
        let mut skip_update = false;
        if portal_camera.plane_mode.is_some() {
            // behindness is positive when the main camera is behind the portal plane
            let behindness = portal_transform.forward().dot((main_camera_transform.translation - portal_transform.translation).normalize());

            if portal_camera.plane_mode == Some(Face::Back) && behindness > PLANE_MODE_TRIGGER
                || portal_camera.plane_mode == Some(Face::Front) && behindness < -PLANE_MODE_TRIGGER {
                // TOFIX makes the app very jerky, why?
                camera.is_active = false;
                skip_update = true;
            }
            else {
                camera.is_active = true;
            }
        } // TODO deactivate camera when looking away from the portal
        if !skip_update {
            // Resize the image if needed
            // TOFIX (mutable access to the image makes it not update by the PortalCamera anymore for some reason)
            // Probably relevant
            // https://github.com/bevyengine/bevy/blob/9d1193df6c300dede75b00ab092caa119a7e80ad/examples/shader/post_process_pass.rs
            // https://discord.com/channels/691052431525675048/1019697973933899910/threads/1093930187802017953
            let portal_image = images.get(&portal_camera.image).unwrap();
            let portal_image_size = portal_image.size();
            let main_camera_viewport_size = get_viewport_size(main_camera, &primary_window_query, &windows_query, &mut images);

            if (portal_image_size.x / portal_image_size.y) != ((main_camera_viewport_size.x as f32)/(main_camera_viewport_size.y as f32)) {
                let size = Extent3d {
                    width: main_camera_viewport_size.x,
                    height: main_camera_viewport_size.y,
                    ..default()
                };
                let portal_image = images.get_mut(&portal_camera.image).unwrap(); // This doesn't work :(
                portal_image.texture_descriptor.size = size;
                portal_image.resize(size);
            }
            
            // Destination
            let destination_result = destination_query.get(portal_camera.parts.destination);
            if let Err(query_error) = destination_result {
                deal_with_part_query_error(&mut commands, &portal_camera.parts, &strategy, &query_error, "Destination");
                return;
            }
            let destination_transform = destination_result.unwrap();
            let destination_transform = &destination_transform.compute_transform();

            // TODO check if any portal part transform changed before updating the portal camera one

            // Move portal camera
            let new_portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, destination_transform);
            portal_camera_transform.set(Box::new(new_portal_camera_transform)).unwrap();
            // We update the global transform manually here for two reasons:
            // 1) This system is run after global transform propagation
            // so if we don't do that the portal camera's global transform would be lagging behind one frame
            // 2) it is not updated nor propagated automatically in Bevy 0.10.1, for some reason
            // (I tried compying the queries of propagate_transforms and have the portal camera and its child in the results).
            // Since the set-up of TransformPlugin will change in Bevy 0.11, this is a WONTFIX until Bevy 0.11
            portal_camera_global_transform.set(Box::new(GlobalTransform::from(new_portal_camera_transform))).unwrap();
        }
    }
}

/// Helper function to get the size of the viewport of the main camera, to be used for the size of the render image.
fn get_viewport_size(
    main_camera: &Camera,
    primary_window_query: &Query<&Window, With<PrimaryWindow>>,
    windows_query: &Query<&Window>,
    images: &mut ResMut<Assets<Image>>,
) -> UVec2 {
    match main_camera.viewport.as_ref() {
        |Some(viewport) => viewport.physical_size,
        |None => match &main_camera.target {
            RenderTarget::Window(window_ref) => {
                let window = match window_ref {
                    WindowRef::Primary => primary_window_query.get_single().unwrap(),
                    WindowRef::Entity(entity) => windows_query.get(entity.clone()).unwrap()
                };
                UVec2::new(window.physical_width(),window.physical_height())
            },
            RenderTarget::Image(handle) => images.get(handle).unwrap().size().as_uvec2()
        }
    }
}

/// Helper function to get the transform to change the main camera's transform into the portal camera's transform.
fn get_portal_camera_transform(main_camera_transform: &Transform, portal_transform: &Transform, destination_transform: &Transform) -> Transform {
    let portal_camera_translation = main_camera_transform.translation - portal_transform.translation + destination_transform.translation;
    let rotation = portal_transform.rotation.inverse().mul_quat(destination_transform.rotation);
    let mut portal_camera_transform = Transform {
        translation: portal_camera_translation,
        rotation: main_camera_transform.rotation,
        scale: main_camera_transform.scale
    };
    portal_camera_transform.rotate_around(destination_transform.translation, rotation);
    portal_camera_transform
}

/// Despawns portal parts according to a strategy
pub fn despawn_portal_parts (
    commands: &mut Commands,
    parts: &PortalParts,
    strategy: &PortalPartsDespawnStrategy,
) {
    despawn_portal_parts_with_message(commands, parts, strategy,
        "is a part of portal parts being despawned but should have been despawned before",
    );
}

fn deal_with_part_query_error (
    commands: &mut Commands,
    parts: &PortalParts,
    strategy: &PortalPartsDespawnStrategy,
    query_error: &QueryEntityError,
    name_of_part: &str
) {
    let error_message = match query_error {
        QueryEntityError::QueryDoesNotMatch(entity) =>
            format!("is a part of portal parts where {} #{} is missing key components", name_of_part, entity.index()),
        QueryEntityError::NoSuchEntity(entity) =>
            format!("is a part of portal parts where {} #{} has despawned", name_of_part, entity.index()),
        QueryEntityError::AliasedMutability(entity) => // No idea what this means
            format!("is a part of portal parts where's {} #{} mutability is aliased", name_of_part, entity.index()),
    };
    despawn_portal_parts_with_message(commands, parts, strategy, &error_message);
}

fn despawn_portal_parts_with_message (
    commands: &mut Commands,
    parts: &PortalParts,
    strategy: &PortalPartsDespawnStrategy,
    error_message: &str,
) {
    despawn_portal_part(commands, parts.portal_camera, &strategy.portal_camera, error_message, "Portal Camera");
    despawn_portal_part(commands, parts.destination, &strategy.destination, error_message, "Destination");
    despawn_portal_part(commands, parts.portal, &strategy.portal, error_message, "Portal");
    despawn_portal_part(commands, parts.main_camera, &strategy.main_camera, error_message, "Main Camera");
}

fn despawn_portal_part (
    commands: &mut Commands,
    entity: Entity,
    strategy: &PortalPartDespawnStrategy,
    error_message: &str,
    entity_type: &str,
) {
    if strategy.should_despawn() {
        if let Some(mut camera_commands) = commands.get_entity(entity) {
            if strategy.should_warn() {
                warn!("{entity_type} {error_message}");
            }
            if strategy.should_despawn_children() {
                camera_commands.despawn_descendants();
            }
            camera_commands.despawn();
        }
    }
    else if strategy.should_panic() {
        panic!("{entity_type} {error_message}");
    }
}

pub(super) fn check_portal_camera_despawn(
    mut commands: Commands,
    strategy: Res<PortalPartsDespawnStrategy>,
    portal_camera_query: Query<(&PortalCamera, &Transform, &GlobalTransform, &Camera)>,
    portal_query: Query<&Portal>,
    destination_query: Query<&PortalDestination>,
) {
    for portal in portal_query.iter() {
        if let Err(query_error) = portal_camera_query.get(portal.parts.portal_camera) {
            deal_with_part_query_error(&mut commands, &portal.parts, &strategy, &query_error, "Portal Camera");
        }
    }
    for destination in destination_query.iter() {
        if let Err(query_error) = portal_camera_query.get(destination.parts.portal_camera) {
            deal_with_part_query_error(&mut commands, &destination.parts, &strategy, &query_error, "Portal Camera");
        }
    }
}