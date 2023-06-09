///! System and helpers for the update of portal cameras

use bevy_app::prelude::*;
use bevy_asset::Assets;
use bevy_ecs::{
    prelude::*,
    system::SystemParam
};
use bevy_math::UVec2;
use bevy_reflect::Reflect;
use bevy_render::{
    prelude::*,
    render_resource::{
        Extent3d,
        Face,
    },
    camera::RenderTarget,
};
use bevy_transform::prelude::*;
use bevy_window::{
    PrimaryWindow,
    Window,
    WindowRef,
};

use super::*;

const PLANE_MODE_TRIGGER: f32 = 0.2;

/// Add the update logic to [PortalsPlugin]
pub(super) fn build_update(app: &mut App) {
    app.add_system(update_portal_cameras.in_base_set(CoreSet::Last));
}

/// Moves the [PortalCamera] to follow the main camera relative to the portal and the destination.
#[allow(clippy::too_many_arguments)]
pub fn update_portal_cameras(
    mut commands: Commands,
    strategy: Res<PortalPartsDespawnStrategy>,
    mut portal_cameras: Query<(&PortalCamera, &mut Transform, &mut GlobalTransform, &mut Camera)>,
    main_camera_query: Query<(&GlobalTransform, &Camera), Without<PortalCamera>>,
    portal_query: Query<&GlobalTransform,(With<Portal>, Without<Camera>)>,
    destination_query: Query<&GlobalTransform, (With<PortalDestination>, Without<Camera>)>,
    mut resize_params: PortalImageSizeParams,
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

        let camera_should_update = camera_should_update(portal_camera, portal_transform, main_camera_transform);
        if !camera_should_update {
            // TOFIX https://github.com/bevyengine/bevy/issues/8777
            //camera.is_active = false;
        }
        else {
            camera.is_active = true;

            resize_image_if_needed(portal_camera, main_camera, &mut resize_params);
            
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

/// Checks if the portal camera should update (move and render)
/// 
/// This will return false if the main camera is "behind" a portal visible only from the front, or looking away from the portal
fn camera_should_update(
    portal_camera: &PortalCamera,
    portal_transform: &Transform,
    main_camera_transform: &Transform,
) -> bool {
    if portal_camera.plane_mode.is_some() {
        // behindness is positive when the main camera is behind the portal plane
        let behindness = portal_transform.forward().dot((main_camera_transform.translation - portal_transform.translation).normalize());

        if portal_camera.plane_mode == Some(Face::Back) && behindness > PLANE_MODE_TRIGGER
            || portal_camera.plane_mode == Some(Face::Front) && behindness < -PLANE_MODE_TRIGGER {
            return false;
        }
    } // TODO deactivate camera when looking away from the portal
    true
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

/// Helper function to get the size of the viewport of the main camera, to be used for the size of the render image.
pub(super) fn get_viewport_size (
    main_camera: &Camera,
    PortalImageSizeParams {
        images,
        primary_window_query,
        windows_query,
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
            RenderTarget::Image(handle) => images.get(handle).unwrap().size().as_uvec2()
        }
    }
}

/// [SystemParam] needed to compute the size of the portal image
#[derive(SystemParam)]
pub struct PortalImageSizeParams<'w, 's> {
    pub(super) images: ResMut<'w, Assets<Image>>,
    primary_window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    windows_query: Query<'w, 's, &'static Window>,
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