#![cfg(feature = "picking_backend")]
//! Module to pick entities through portals.
//! This requires the feature `picking_backend`.

use crate::*;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use bevy_picking::{backend::prelude::*, focus::HoverMap, pointer::Location};
use bevy_render::camera::NormalizedRenderTarget;
use bevy_transform::prelude::*;
use uuid::Uuid;

pub(crate) struct PortalPickingBackendPlugin;

impl Plugin for PortalPickingBackendPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, pick_through_portals.in_set(PickSet::Backend));
        app.add_observer(add_pointer);
    }
}

fn add_pointer(
    trigger: Trigger<OnAdd, PortalCamera>,
    mut commands: Commands,
    portal_cameras: Query<&PortalCamera>,
) {
    let portal_camera_entity = trigger.entity();
    let portal_camera = portal_cameras.get(portal_camera_entity).unwrap();

    commands.entity(portal_camera_entity).insert((
        PointerId::Custom(Uuid::new_v4()),
        PointerLocation::new(Location {
            target: NormalizedRenderTarget::Image(portal_camera.image.clone()),
            position: Vec2::ZERO,
        }),
    ));
}

pub fn pick_through_portals(hovers: Res<HoverMap>, portals: Query<(&Portal, &GlobalTransform)>) {
    for (_pointer_id, hits) in hovers.iter() {
        for (entity, hit_data) in hits {
            if portals.contains(*entity) {}
        }
        /*if let Some((portal_hit, portal_hit_data, portal_transform)) = hit.picks.iter().find_map(
            |(entity, hit_data)|
            if let Ok((portal, portal_transform)) = portals.get(*entity) {
                if portal.parts.main_camera == hit_data.camera {
                    Some((portal, hit_data, portal_transform))
                } else {
                    None
                }
            } else {
                None
            }
        ) {
            hits_writer.send(PointerHits {

            })
        }*/
    }
}
