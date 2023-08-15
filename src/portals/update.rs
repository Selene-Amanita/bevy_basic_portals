///! System and helpers for the update of portal cameras

use bevy_app::prelude::*;
use bevy_asset::Assets;
use bevy_ecs::{
    prelude::*,
    system::SystemParam
};
use bevy_math::{UVec2, Quat, Vec3};
use bevy_render::{
    prelude::*,
    render_resource::Extent3d,
    camera::{RenderTarget, CameraProjection, ManualTextureViews},
    primitives::{Frustum, HalfSpace},
};
use bevy_transform::prelude::*;
use bevy_window::{
    PrimaryWindow,
    Window,
    WindowRef,
};

use super::*;

/// Add the update logic to [PortalsPlugin]
pub(super) fn build_update(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update_portal_cameras
            // TODO: once we can use PortalProjection, we can ignore update_frusta
            // see https://github.com/bevyengine/bevy/pull/9226
            //.after(bevy_transform::TransformSystem::TransformPropagate)
            .after(bevy_render::view::update_frusta::<Projection>)
    );
}

/// Moves the [PortalCamera] to follow the main camera relative to the portal and the destination.
#[allow(clippy::too_many_arguments)]
pub fn update_portal_cameras(
    mut commands: Commands,
    strategy: Res<PortalPartsDespawnStrategy>,
    mut portal_cameras: Query<(&PortalCamera, &mut Transform, &mut GlobalTransform, &mut Frustum, &Projection), With<Camera>>, // TODO: use PortalProjection in the future
    main_camera_query: Query<(Ref<GlobalTransform>, &Camera), Without<PortalCamera>>,
    portal_query: Query<Ref<GlobalTransform>, (With<Portal>, Without<Camera>)>,
    destination_query: Query<Ref<GlobalTransform>, (With<PortalDestination>, Without<Camera>)>,
    mut resize_params: PortalImageSizeParams,
) {
    // For every portal camera
    for (
        portal_camera,
        mut portal_camera_transform,
        mut portal_camera_global_transform,
        mut frustum,
        projection,
    ) in portal_cameras.iter_mut() {

        // Main Camera
        let main_camera_result = main_camera_query.get(portal_camera.parts.main_camera);
        if let Err(query_error) = main_camera_result {
            deal_with_part_query_error(&mut commands, &portal_camera.parts, &strategy, &query_error, "Main Camera");
            return;
        }
        let (main_camera_global_transform, main_camera) = main_camera_result.unwrap();

        // Portal
        let portal_result = portal_query.get(portal_camera.parts.portal);
        if let Err(query_error) = portal_result {
            deal_with_part_query_error(&mut commands, &portal_camera.parts, &strategy, &query_error, "Portal");
            return;
        }
        let portal_global_transform = portal_result.unwrap();
        
        // Destination
        let destination_result = destination_query.get(portal_camera.parts.destination);
        if let Err(query_error) = destination_result {
            deal_with_part_query_error(&mut commands, &portal_camera.parts, &strategy, &query_error, "Destination");
            return;
        }
        let destination_global_transform = destination_result.unwrap();

        resize_image_if_needed(portal_camera, main_camera, &mut resize_params);

        // Needed for update frustum later because of update_frusta
        let destination_transform = &destination_global_transform.compute_transform();

        if portal_global_transform.is_changed()
        || destination_global_transform.is_changed()
        || main_camera_global_transform.is_changed() {
            let portal_transform = &portal_global_transform.compute_transform();
            let main_camera_transform = &main_camera_global_transform.compute_transform();

            // Move portal camera
            let new_portal_camera_transform = get_portal_camera_transform(main_camera_transform, portal_transform, destination_transform);
            *portal_camera_transform = new_portal_camera_transform;
            // We update the global transform manually here for two reasons:
            // 1) This system is run after global transform propagation
            // so if we don't do that the portal camera's global transform would be lagging behind one frame
            // 2) The portal camera should not be in a hierarchy in theory (?)
            *portal_camera_global_transform = GlobalTransform::from(new_portal_camera_transform);
        }

        // We can't put it in the block above because update_frusta will update it
        // when the portal camera's global transform is updated, which will happen in the next tick
        if portal_camera_global_transform.is_changed() {
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
) {
    // TOFIX (mutable access to the image makes it not update by the PortalCamera anymore for some reason)
    // see https://github.com/bevyengine/bevy/issues/8767
    // Probably relevant
    // https://github.com/bevyengine/bevy/blob/9d1193df6c300dede75b00ab092caa119a7e80ad/examples/shader/post_process_pass.rs
    // https://discord.com/channels/691052431525675048/1019697973933899910/threads/1093930187802017953
    let portal_image = size_params.images.get(&portal_camera.image).unwrap();
    let portal_image_size = portal_image.size();
    let main_camera_viewport_size = get_viewport_size(main_camera, size_params);

    if (portal_image_size.x / portal_image_size.y) != ((main_camera_viewport_size.x as f32)/(main_camera_viewport_size.y as f32)) {
        let size = Extent3d {
            width: main_camera_viewport_size.x,
            height: main_camera_viewport_size.y,
            ..Extent3d::default()
        };
        let portal_image = size_params.images.get_mut(&portal_camera.image).unwrap(); // This doesn't work :(
        portal_image.texture_descriptor.size = size;
        portal_image.resize(size);
    }
}

/// Get the [Frustum] for the [PortalCamera] from the [PortalProjection] and
/// modifying it depending on the [PortalMode].
fn get_frustum(
    portal_camera: &PortalCamera,
    portal_camera_transform: &Transform,
    destination_transform: &Transform,
    projection: &Projection, // TODO: use PortalProjection in the future
) -> Frustum {
    let view_projection =
        projection.get_projection_matrix() * portal_camera_transform.compute_matrix().inverse();

    let mut frustum = Frustum::from_view_projection_custom_far(
        &view_projection,
        &portal_camera_transform.translation,
        &portal_camera_transform.back(),
        projection.far(),
    );

    match portal_camera.portal_mode {
        PortalMode::MaskedImageHalfSpaceFrustum(Some(half_space)) => {
            let rot = Quat::from_rotation_arc(
                Vec3::NEG_Z,
                destination_transform.forward(),
            );
            let near_half_space_normal = rot.mul_vec3(half_space.normal().into());

            let dot = destination_transform.translation.dot(near_half_space_normal.normalize());
            let near_half_space_distance = -(dot + half_space.d());

            frustum.half_spaces[4] = HalfSpace::new(near_half_space_normal.extend(near_half_space_distance))
        }
        PortalMode::MaskedImageHalfSpaceFrustum(None) => {
            let near_half_space_normal = destination_transform.forward();
            let near_half_space_distance = - destination_transform.translation.dot(near_half_space_normal);
            frustum.half_spaces[4] = HalfSpace::new(near_half_space_normal.extend(near_half_space_distance))
        }
        _ => ()
    };

    frustum
}

/// Helper function to get the size of the viewport of the main camera, to be used for the size of the render image.
pub(super) fn get_viewport_size (
    main_camera: &Camera,
    PortalImageSizeParams {
        images,
        primary_window_query,
        windows_query,
        texture_views,
    }: &PortalImageSizeParams,
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
            RenderTarget::Image(handle) => images.get(handle).unwrap().size().as_uvec2(),
            RenderTarget::TextureView(handle) => texture_views.get(handle).unwrap().size
        }
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