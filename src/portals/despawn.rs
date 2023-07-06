///! System and helpers for the update of portal cameras

use bevy_app::App;
use bevy_ecs::{
    prelude::*,
    query::QueryEntityError,
    system::{
        Command,
        EntityCommand,
        SystemState
    },
};
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_render::camera::Camera;
use bevy_transform::prelude::*;
use tracing::warn;

use super::*;

/// Add the despawn logic to [PortalsPlugin]
pub(super) fn build_despawn(app: &mut App, despawn_strategy: Option<PortalPartsDespawnStrategy>, should_check_portal_camera_despawn: bool) {
    app.register_type::<PortalPartsDespawnStrategy>();

    if let Some(despawn_strategy) = despawn_strategy {
        app.insert_resource(despawn_strategy);
    } else {
        app.init_resource::<PortalPartsDespawnStrategy>();
    }

    if should_check_portal_camera_despawn {
        app.add_system(check_portal_camera_despawn);
    }
}

/// [Command] to despawn portal parts according to a strategy
pub struct DespawnPortalPartsCommand {
    portal_parts: PortalParts,
    strategy: PortalPartsDespawnStrategy,
}

impl Command for DespawnPortalPartsCommand {
    fn write(self, world: &mut World) {
        let mut system_state = SystemState::<Commands>::new(world);
        let mut commands = system_state.get_mut(world);

        despawn_portal_parts(&mut commands, &self.portal_parts, &self.strategy);

        system_state.apply(world);
    }
}

/// [EntityCommand] to despawn the portal parts linked to the entity, according to a strategy
#[derive(Default)]
pub struct DespawnPortalPartsEntityCommand(PortalPartsDespawnStrategy);

impl EntityCommand for DespawnPortalPartsCommand {
    fn write(self, entity: Entity, world: &mut World) {
        let mut system_state = SystemState::<(
            Commands,
            Query<&Portal>,
            Query<&PortalDestination>,
            Query<&PortalCamera>,
        )>::new(world);
        let (
            mut commands,
            portal_query,
            portal_destination_query,
            portal_camera_query
        ) = system_state.get_mut(world);

        let portal_parts =
            portal_query.get(entity).map_or_else(|_|
                portal_destination_query.get(entity).map_or_else(|_|
                    portal_camera_query.get(entity).map_or_else(|_|
                        None,
                    |c| Some(&c.parts)),
                |d| Some(&d.parts)),
            |p| Some(&p.parts));

        if let Some(portal_parts) = portal_parts {
            despawn_portal_parts(&mut commands, portal_parts, &self.strategy);
        }
        else {
            warn!("DespawnPortalPartsEntityCommand called on entity {} which is not a portal part", entity.index())
        }

        system_state.apply(world);
    }
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

/// [System] which checks if a [PortalCamera] despawned or has the wrong components, but the [Portal] or [PortalDestination] still exist
pub fn check_portal_camera_despawn(
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

/// Helper function to deal with "missing" portal parts,
/// see [PortalsPlugin](struct.PortalsPlugin.html#structfield.despawn_strategy)
pub(super) fn deal_with_part_query_error (
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
        QueryEntityError::AliasedMutability(entity) => // Shouldn't happen
            format!("is a part of portal parts where's {} #{} is accessed twice mutably", name_of_part, entity.index()),
    };
    despawn_portal_parts_with_message(commands, parts, strategy, &error_message);
}