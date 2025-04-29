//! System and helpers for the update of portal cameras

use bevy_app::prelude::*;
use bevy_ecs::{
    prelude::*,
    query::QueryEntityError,
    system::{EntityCommand, SystemState},
};
use tracing::warn;

use super::*;

/// Add the despawn logic to [PortalsPlugin]
pub(super) fn build_despawn(
    app: &mut App,
    despawn_strategy: Option<PortalPartsDespawnStrategy>,
    should_check_portal_parts_back_reference: bool,
) {
    app.register_type::<PortalPartsDespawnStrategy>();

    if let Some(despawn_strategy) = despawn_strategy {
        app.insert_resource(despawn_strategy);
    } else {
        app.init_resource::<PortalPartsDespawnStrategy>();
    }

    if should_check_portal_parts_back_reference {
        app.add_systems(Update, check_portal_parts_back_references);
    }
}

/// [Command] to despawn portal parts according to a strategy
pub struct DespawnPortalPartsCommand {
    portal_parts: PortalParts,
    strategy: PortalPartsDespawnStrategy,
}

impl Command for DespawnPortalPartsCommand {
    fn apply(self, world: &mut World) {
        let mut system_state = SystemState::<Commands>::new(world);
        let mut commands = system_state.get_mut(world);

        despawn_portal_parts(&mut commands, &self.portal_parts, &self.strategy);

        system_state.apply(world);
    }
}

/// [EntityCommand] to despawn the portal parts linked to the entity, according to a strategy
#[derive(Default)]
pub struct DespawnPortalPartsEntityCommand(PortalPartsDespawnStrategy);

impl EntityCommand for DespawnPortalPartsEntityCommand {
    fn apply(self, mut entity_world: EntityWorldMut) {
        let entity = entity_world.id();
        entity_world.world_scope(move |world: &mut World| {
            let mut system_state =
                SystemState::<(Commands, Query<&PortalPart>, Query<&PortalParts>)>::new(world);
            let (mut commands, portal_part_query, portal_parts_query) = system_state.get_mut(world);

            let portal_parts = portal_part_query.get(entity).map_or_else(
                |_| portal_parts_query.get(entity).ok(),
                |p| portal_parts_query.get(p.parts).ok(),
            );

            if let Some(portal_parts) = portal_parts {
                despawn_portal_parts(&mut commands, portal_parts, &self.0);
            } else {
                warn!(
                    "DespawnPortalPartsEntityCommand called on entity {} which is not a portal part nor a portal parts entity, or is a portal part but referencing a despawned portal parts",
                    entity.index()
                )
            }

            system_state.apply(world);
        });
    }
}

/// Despawns portal parts according to a strategy
pub fn despawn_portal_parts(
    commands: &mut Commands,
    parts: &PortalParts,
    strategy: &PortalPartsDespawnStrategy,
) {
    despawn_portal_parts_with_message(
        commands,
        parts,
        strategy,
        "is a part of portal parts being despawned but should have been despawned before",
    );
}

fn despawn_portal_parts_with_message(
    commands: &mut Commands,
    parts: &PortalParts,
    strategy: &PortalPartsDespawnStrategy,
    error_message: &str,
) {
    despawn_portal_part(
        commands,
        parts.portal_camera,
        strategy.portal_camera,
        error_message,
        "Portal Camera",
    );
    despawn_portal_part(
        commands,
        parts.destination,
        strategy.destination,
        error_message,
        "Destination",
    );
    despawn_portal_part(
        commands,
        parts.portal,
        strategy.portal,
        error_message,
        "Portal",
    );
    despawn_portal_part(
        commands,
        parts.main_camera,
        strategy.main_camera,
        error_message,
        "Main Camera",
    );
}

fn despawn_portal_part(
    commands: &mut Commands,
    entity: Entity,
    strategy: PortalPartDespawnStrategy,
    error_message: &str,
    entity_type: &str,
) {
    if strategy.should_despawn() {
        if let Ok(mut camera_commands) = commands.get_entity(entity) {
            if strategy.should_warn() {
                warn!("{entity_type} {error_message}");
            }
            if strategy.should_despawn_children() {
                camera_commands.despawn_related::<Children>();
            }
            camera_commands.despawn();
        }
    } else if strategy.should_panic() {
        panic!("{entity_type} {error_message}");
    }
}

/// [System] which checks if a [PortalPart] is referencing back a [PortalParts] entity which has been despawned.
pub fn check_portal_parts_back_references(
    mut commands: Commands,
    strategy: Res<PortalPartsDespawnStrategy>,
    portal_part_query: Query<(Entity, &PortalPart)>,
    portal_parts_query: Query<&PortalParts>,
    portal_query: Query<&Portal>,
    destination_query: Query<&PortalDestination>,
    portal_camera_query: Query<&PortalCamera>,
) {
    for (part_entity, part) in portal_part_query.iter() {
        if !portal_parts_query.contains(part.parts) {
            let strategy = if portal_query.contains(part_entity) {
                strategy.portal
            } else if destination_query.contains(part_entity) {
                strategy.destination
            } else if portal_camera_query.contains(part_entity) {
                strategy.portal_camera
            } else {
                warn!(
                    "Portal Part #{} isn't a portal, a destination or a portal camera",
                    part_entity
                );
                continue;
            };

            despawn_portal_part(
                &mut commands,
                part_entity,
                strategy,
                &format!(
                    "#{} has a reference to a PortalParts entity which has been despawned.",
                    part_entity,
                ),
                "Portal Part",
            )
        }
    }
}

/// Helper function to deal with "missing" portal parts,
/// see [PortalsPlugin](struct.PortalsPlugin.html#structfield.despawn_strategy)
pub(super) fn deal_with_part_query_error(
    commands: &mut Commands,
    parts: &PortalParts,
    parts_entity: Entity,
    strategy: &PortalPartsDespawnStrategy,
    query_error: QueryEntityError,
    name_of_part: &str,
) {
    let error_message = match query_error {
        QueryEntityError::QueryDoesNotMatch(entity, _world) => format!(
            "is a part of portal parts {} where {} #{} is missing key components",
            parts_entity,
            name_of_part,
            entity.index() // TODO: reproduce format_archetype's behavior
        ),
        QueryEntityError::EntityDoesNotExist(error) => format!(
            "is a part of portal parts {} where {} #{} has despawned (details: {})",
            parts_entity,
            name_of_part,
            error.entity.index(),
            error.details,
        ),
        QueryEntityError::AliasedMutability(entity) =>
        // Shouldn't happen
        {
            format!(
                "is a part of portal parts {} where {} #{} is accessed twice mutably",
                parts_entity,
                name_of_part,
                entity.index()
            )
        }
    };
    despawn_portal_parts_with_message(commands, parts, strategy, &error_message);
}
