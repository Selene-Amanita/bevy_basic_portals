//! System and helpers for the update of portal cameras

use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_image::Image;
use bevy_math::{Quat, UVec2, Vec3};
use bevy_pbr::MeshMaterial3d;
use bevy_render::{
    camera::{CameraProjection, ManualTextureViews, RenderTarget},
    prelude::*,
    primitives::{Frustum, HalfSpace},
    render_resource::Extent3d,
};
use bevy_transform::prelude::*;
use bevy_window::{PrimaryWindow, Window, WindowRef};
use tracing::warn;

use super::*;

/// Add the update logic to [PortalsPlugin]
pub(super) fn build_update(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update_portal_cameras.after(bevy_transform::TransformSystem::TransformPropagate),
    );
}

/// Moves the [PortalCamera] to follow the main camera relative to the portal and the destination.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_portal_cameras(
    mut commands: Commands,
    strategy: Res<PortalPartsDespawnStrategy>,
    portal_parts_query: Query<(Entity, &PortalParts)>,
    mut portal_cameras: Query<
        (
            &PortalCamera,
            &mut Transform,
            &mut GlobalTransform,
            &mut Frustum,
            &PortalProjection,
        ),
        With<Camera>,
    >,
    main_camera_query: Query<(Ref<GlobalTransform>, &Camera), Without<PortalCamera>>,
    portal_query: Query<
        (Ref<GlobalTransform>, &MeshMaterial3d<PortalMaterial>),
        (With<Portal>, Without<Camera>),
    >,
    destination_query: Query<(Ref<GlobalTransform>, &PortalDestination), (With<PortalDestination>, Without<Camera>)>,
    mut resize_params: PortalImageSizeParams,
    mut materials: ResMut<Assets<PortalMaterial>>,
) {
    // For every portal parts
    for (portal_parts_entity, portal_parts) in portal_parts_query.iter() {
        // Portal camera
        let (
            portal_camera,
            mut portal_camera_transform,
            mut portal_camera_global_transform,
            mut frustum,
            projection,
        ) = match portal_cameras.get_mut(portal_parts.portal_camera) {
            Ok(result) => result,
            Err(query_error) => {
                deal_with_part_query_error(
                    &mut commands,
                    portal_parts,
                    portal_parts_entity,
                    &strategy,
                    query_error,
                    "Portal Camera",
                );
                return;
            }
        };

        // Main Camera
        let (main_camera_global_transform, main_camera) = match main_camera_query.get(portal_parts.main_camera) {
            Ok(result) => result,
            Err(query_error) => {
                deal_with_part_query_error(
                    &mut commands,
                    portal_parts,
                    portal_parts_entity,
                    &strategy,
                    query_error,
                    "Main Camera",
                );
                return;
            }
        };
        
        // Portal
        let (portal_global_transform, portal_material) = match portal_query.get(portal_parts.portal) {
            Ok(result) => result,
            Err(query_error) => {
                deal_with_part_query_error(
                    &mut commands,
                    portal_parts,
                    portal_parts_entity,
                    &strategy,
                    query_error,
                    "Portal",
                );
                return;
            }
        };

        // Destination
        let (destination_global_transform, destination) = match destination_query.get(portal_parts.destination) {
            Ok(result) => result,
            Err(query_error) => {
                deal_with_part_query_error(
                    &mut commands,
                    portal_parts,
                    portal_parts_entity,
                    &strategy,
                    query_error,
                    "Destination",
                );
                return;
            }
        };

        // Resize image
        let portal_image_resized = resize_image_if_needed(
            portal_camera,
            main_camera,
            &mut resize_params,
            portal_material,
            &mut materials,
        );

        // Needed for update frustum later because of update_frusta
        let destination_transform = &destination_global_transform.compute_transform();

        let should_update_transform = portal_global_transform.is_changed()
            || destination_global_transform.is_changed()
            || main_camera_global_transform.is_changed();

        if should_update_transform {
            // Move portal camera
            let new_portal_camera_global_transform = get_portal_camera_transform(
                &main_camera_global_transform,
                &portal_global_transform,
                &destination_global_transform,
                destination.mirror_x,
                destination.mirror_y,
            );
            *portal_camera_transform = new_portal_camera_global_transform.into();
            // We update the global transform manually here for two reasons:
            // 1) This system is run after global transform propagation
            // so if we don't do that the portal camera's global transform would be lagging behind one frame
            // 2) The portal camera should not be in a hierarchy in theory (?)
            *portal_camera_global_transform = new_portal_camera_global_transform;
        }

        if portal_image_resized || should_update_transform {
            // Update frustum
            let new_frustum = get_frustum(
                portal_camera,
                &portal_camera_transform,
                destination_transform,
                projection,
            );
            *frustum = new_frustum;
        }

        // TODO: Check if camera should update
    }
}

/// Resize the image used to render a portal, if needed
fn resize_image_if_needed(
    portal_camera: &PortalCamera,
    main_camera: &Camera,
    size_params: &mut PortalImageSizeParams,
    portal_material: &Handle<PortalMaterial>,
    materials: &mut Assets<PortalMaterial>,
) -> bool {
    let portal_image = size_params.images.get(&portal_camera.image).unwrap();
    let portal_image_size = portal_image.size();
    let Some(main_camera_viewport_size) = get_viewport_size(main_camera, size_params) else {
        warn!("Viewport size not found, skipping portal resize");
        return false;
    };

    let resize = portal_image_size.x != main_camera_viewport_size.x
        || portal_image_size.y != main_camera_viewport_size.y;
    if resize {
        let size = Extent3d {
            width: main_camera_viewport_size.x,
            height: main_camera_viewport_size.y,
            ..Extent3d::default()
        };
        if let (Some(portal_image), Some(_)) = (
            size_params.images.get_mut(&portal_camera.image),
            // This is needed so that the material is aware the image changed,
            // see https://github.com/bevyengine/bevy/issues/8767
            materials.get_mut(portal_material),
        ) {
            portal_image.texture_descriptor.size = size;
            portal_image.resize(size);
        } else {
            warn!("No portal image or material.");
        }
    }

    resize
}

/// Get the [Frustum] for the [PortalCamera] from the [PortalProjection] and
/// modifying it depending on the [PortalMode].
fn get_frustum(
    portal_camera: &PortalCamera,
    portal_camera_transform: &Transform,
    destination_transform: &Transform,
    projection: &PortalProjection,
) -> Frustum {
    let view_projection =
        projection.get_clip_from_view() * portal_camera_transform.compute_matrix().inverse();

    let mut frustum = Frustum::from_clip_from_world_custom_far(
        &view_projection,
        &portal_camera_transform.translation,
        &portal_camera_transform.back(),
        projection.far(),
    );

    match portal_camera.portal_mode {
        PortalMode::MaskedImageHalfSpaceFrustum(Some(half_space)) => {
            let rot = Quat::from_rotation_arc(
                Vec3::NEG_Z,
                destination_transform.forward().normalize_or_zero(),
            );
            let near_half_space_normal = rot.mul_vec3(half_space.normal().into());

            let dot = destination_transform
                .translation
                .dot(near_half_space_normal.normalize());
            let near_half_space_distance = -(dot + half_space.d());

            frustum.half_spaces[4] =
                HalfSpace::new(near_half_space_normal.extend(near_half_space_distance))
        }
        PortalMode::MaskedImageHalfSpaceFrustum(None) => {
            let near_half_space_normal = destination_transform.forward();
            let near_half_space_distance = -destination_transform
                .translation
                .dot(near_half_space_normal.normalize_or_zero());
            frustum.half_spaces[4] =
                HalfSpace::new(near_half_space_normal.extend(near_half_space_distance))
        }
        _ => (),
    };

    frustum
}

/// Helper function to get the size of the viewport of the main camera, to be used for the size of the render image.
pub(super) fn get_viewport_size(
    main_camera: &Camera,
    PortalImageSizeParams {
        images,
        primary_window_query,
        windows_query,
        texture_views,
    }: &PortalImageSizeParams,
) -> Option<UVec2> {
    match main_camera.viewport.as_ref() {
        Some(viewport) => Some(viewport.physical_size),
        None => match &main_camera.target {
            RenderTarget::Window(window_ref) => (match window_ref {
                WindowRef::Primary => primary_window_query.get_single().ok(),
                WindowRef::Entity(entity) => windows_query.get(*entity).ok(),
            })
            .map(|window| UVec2::new(window.physical_width(), window.physical_height())),
            RenderTarget::Image(handle) => images.get(handle).map(|image| image.size()),
            RenderTarget::TextureView(handle) => texture_views
                .get(handle)
                .map(|texture_view| texture_view.size),
        },
    }
}

/// [SystemParam] needed to compute the size of the portal image
#[derive(SystemParam)]
pub struct PortalImageSizeParams<'w, 's> {
    pub(super) images: ResMut<'w, Assets<Image>>,
    primary_window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    windows_query: Query<'w, 's, &'static Window>,
    texture_views: Res<'w, ManualTextureViews>,
}

/// Helper function to get the transform to change the main camera's transform into the portal camera's transform.
fn get_portal_camera_transform(
    main_camera_transform: &GlobalTransform,
    portal_transform: &GlobalTransform,
    destination_transform: &GlobalTransform,
    mirror_x: bool,
    mirror_y: bool,
) -> GlobalTransform {
    let mut portal_camera_global_transform: GlobalTransform = (
        destination_transform.affine()
        * portal_transform.affine().inverse()
        * main_camera_transform.affine()
    ).into();

    if mirror_y {
        let mut transform = portal_camera_global_transform.compute_transform();
        mirror_transform(
            &mut transform,
            destination_transform.translation(),
            destination_transform.right().into()
        );
        portal_camera_global_transform = transform.into();
    }
    if mirror_x {
        let mut transform = portal_camera_global_transform.compute_transform();
        mirror_transform(
            &mut transform,
            destination_transform.translation(),
            destination_transform.up().into()
        );
        portal_camera_global_transform = transform.into();
    }

    portal_camera_global_transform
}

// Mirrors a vector "without origin" (/with the same origin as the mirror's normal)
fn mirror_vec(vec: Vec3, mirror_normal: Vec3) -> Vec3 {
    let vec_proj = vec.project_onto(mirror_normal);
    vec - 2. * vec_proj
}

fn mirror_transform(transform: &mut Transform, mirror_translation: Vec3, mirror_normal: Vec3) {
    transform.translation = mirror_translation + mirror_vec(
        transform.translation - mirror_translation,
        mirror_normal
    );
    transform.look_to(
        mirror_vec(
            transform.forward().into(),
            mirror_normal
        ),
        mirror_vec(
            transform.up().into(),
            mirror_normal
        ),
    );
}
