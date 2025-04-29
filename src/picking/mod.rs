//! Module to pick entities through portals.
//! This requires the feature `picking_backend`.

use crate::*;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use bevy_picking::{
    backend::prelude::*,
    hover::HoverMap,
    pointer::{Location, PointerAction, PointerInput},
};
use bevy_render::camera::NormalizedRenderTarget;
use bevy_transform::prelude::*;
use tracing::debug;
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
    let portal_camera_entity = trigger.target();
    let portal_camera = portal_cameras.get(portal_camera_entity).unwrap();

    commands.entity(portal_camera_entity).insert((
        PointerId::Custom(Uuid::new_v4()),
        PointerLocation::new(Location {
            target: NormalizedRenderTarget::Image(portal_camera.image.clone().into()),
            position: Vec2::ZERO,
        }),
    ));
}

pub fn pick_through_portals(
    hovers: Res<HoverMap>,
    portals: Query<(&PortalPart, &GlobalTransform), With<Portal>>,
    portal_parts: Query<&PortalParts>,
    portal_cameras: Query<(&PointerId, &PointerLocation), With<PortalCamera>>,
    pointer_events: EventWriter<PointerInput>,
) {
    /*for (pointer_id, hits) in hovers.iter() {
        for (entity, hit_data) in hits {
            if let Ok((parts, portal_transform)) = portals.get(*entity) {
                if let Ok(parts) = portal_parts.get(parts.parts) {
                    if let Ok((
                        portal_pointer_id,
                        PointerLocation {
                            location: Some(portal_pointer_location)
                        }
                    )) = portal_cameras.get(parts.portal_camera) {
                        pointer_events.send(PointerInput {
                            pointer_id: *pointer_id,
                            location: Location {
                                target: portal_pointer_location.target.clone(),
                                location:
                            },
                            action: PointerAction {

                            }
                        });
                    } else {
                        debug!("No portal camera found for portal during picking");
                    }
                } else {
                    debug!("No parts found for portal during picking");
                }
            }
        }
    }*/
}
