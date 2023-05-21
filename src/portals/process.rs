///! Components, systems and others for the implementation of portals, or doing something specific with them like manual creation

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        camera::RenderTarget, mesh::MeshVertexBufferLayout,
    },
    reflect::TypeUuid,
    window::*,
    ecs::system::{EntityCommand, SystemState},
    pbr::{MaterialPipelineKey, MaterialPipeline},
};
use std::f32::consts::PI;

use super::api::*;

const PLANE_MODE_TRIGGER: f32 = 0.2;

/// Marker component for the portal.
/// 
/// Will replace [CreatePortal] after [create_portals].
#[derive(Component)]
pub struct Portal;

/// Marker component for the destination.
/// 
/// Will be added to the entity if [CreatePortal]'s destination is [AsPortalDestination::Use]
#[derive(Component)]
pub struct PortalDestination;

/// Component for a portal camera, the camera that is used to see through a portal.
#[derive(Component)]
pub struct PortalCamera {
    pub image: Handle<Image>,
    pub portal: Entity,
    pub destination: Entity,
    pub main_camera: Entity,
    pub plane_mode: Option<Face>
}

/// Marker component for the debug camera when [DebugPortal::show_window] is true.
#[derive(Component)]
pub struct PortalDebugCamera;

/// Material with the portal shader (renders the image without deformation using the mesh as a mask).
#[derive(AsBindGroup, Clone, TypeUuid)]
#[bind_group_data(PortalMaterialKey)]
#[uuid = "4ee9c363-1124-4113-890e-199d81b00281"]
pub struct PortalMaterial {
    #[texture(0)]
    #[sampler(1)]
    color_texture: Option<Handle<Image>>,
    cull_mode: Option<Face>
}

impl Material for PortalMaterial {
    fn fragment_shader() -> ShaderRef {
        "portal.wgsl".into()
    }

    fn specialize(
        _: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PortalMaterialKey {
    cull_mode: Option<Face>,
}

impl From<&PortalMaterial> for PortalMaterialKey {
    fn from(material: &PortalMaterial) -> Self {
        PortalMaterialKey {
            cull_mode: material.cull_mode,
        }
    }
}


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
        let portal_transform = portal_transform.clone();
        let mesh = mesh.clone();

        let portal_create = match self.config {
            Some(config) => config,
            None => world.query::<&CreatePortal>().get(world, id)
                .expect("You must provide a CreatePortal component to the entity or to the CreatePortalCommand itself before using it").clone()
        };

        //let mut queue = bevy::ecs::system::CommandQueue::default();
        //let commands = Commands::new(&mut queue, world);

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
    portal_global_transform: &GlobalTransform,
    portal_mesh: &Handle<Mesh>
) {
    // Get main camera infos
    let (main_camera_entity, main_camera) = 
        if let Some(camera_entity) = create_portal.main_camera {
            main_camera_query.get(camera_entity).unwrap()
        }
        else {
            main_camera_query.get_single().unwrap()
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

    // Modify the portal entity
    let mut portal_entity_command = commands.entity(portal_entity);
    portal_entity_command.insert(portal_material);
    portal_entity_command.insert(Portal);
    portal_entity_command.remove::<CreatePortal>();

    // Create or get the destination entity
    let destination_entity = match create_portal.destination {
        AsPortalDestination::Use(entity) => entity,
        AsPortalDestination::Create(CreatePortalDestination{transform}) => commands.spawn(SpatialBundle{
            transform,
            global_transform: GlobalTransform::from(transform),
            ..default()
        }).id(),
        AsPortalDestination::CreateMirror => {
            let mut mirror_transform = portal_global_transform.compute_transform();
            mirror_transform.rotate_local_axis(Vec3::Y, PI);
            commands.spawn(SpatialBundle{
                transform: mirror_transform,
                global_transform: GlobalTransform::from(mirror_transform),
                ..default()
            }).id()
        },
    };
    commands.entity(destination_entity).insert(PortalDestination);

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
        PortalCamera {
            image: portal_image,
            portal: portal_entity,
            destination: destination_entity,
            main_camera: main_camera_entity,
            plane_mode: create_portal.plane_mode
        },
        VisibilityBundle {
            visibility: Visibility::Hidden,
            ..default()
        },
        create_portal.render_layer
    )).id();

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
pub fn update_portal_cameras(
    mut images: ResMut<Assets<Image>>,
    mut portal_cameras: Query<(&PortalCamera, &mut Transform, &mut GlobalTransform, &mut Camera)>,
    portal_query: Query<&GlobalTransform,(With<Portal>, Without<Camera>)>,
    destination_query: Query<&GlobalTransform, (With<PortalDestination>, Without<Camera>)>,
    main_camera_query: Query<(&GlobalTransform, &Camera), Without<PortalCamera>>,
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    windows_query: Query<&Window>,
) {
    for (portal_camera, mut portal_camera_transform, mut portal_camera_global_transform, mut camera)
        in portal_cameras.iter_mut() {
        let (main_camera_transform, main_camera) = main_camera_query.get(portal_camera.main_camera).unwrap();
        let main_camera_transform = &main_camera_transform.compute_transform();

        let portal_transform = portal_query.get(portal_camera.portal).unwrap();
        let portal_transform = &portal_transform.compute_transform();

        let mut skip_update = false;
        if portal_camera.plane_mode.is_some() {
            // positive when the main camera is behind the portal plane
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
        } // TOFIX deactivate camera when looking away from the portal
        if !skip_update {
            // TOFIX Resize (mutable access to the image makes it not update by the PortalCamera anymore for some reason)
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
            
            let destination_transform = destination_query.get(portal_camera.destination).unwrap();
            let destination_transform = &destination_transform.compute_transform();

            // Move camera
            let new_portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, destination_transform);
            portal_camera_transform.set(Box::new(new_portal_camera_transform)).unwrap();
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
                    WindowRef::Entity(entity) => windows_query.get(entity.to_owned()).unwrap()
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