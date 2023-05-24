///! Components and structs to create portals without caring about their implementation

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        view::RenderLayers,
    },
    transform::TransformSystem
};

use super::process::*;

/// Adds support for portals to a bevy App.
pub struct PortalsPlugin {
    /// Whether and when to check for entities with [CreatePortal] components to create a portal.
    /// 
    /// Defaults to [PortalsCheckMode::AlwaysCheck].
    pub check_create: PortalsCheckMode,
    /// If true, should add a system to check if a [PortalCamera] despawned or has the wrong components
    pub check_portal_camera_despawn: bool,
    /// What to do when there is a problem getting a [PortalParts]
    /// 
    /// Can happen when :
    /// - a part (main camera, [Portal], [PortalDestination]) has despawned but the [PortalCamera] still exists,
    /// - a part is missing a key component (see [update_portal_cameras]'s implementation).
    /// - check_portal_camera_despawn is true and a portal camera has despawned or missing a key component but the [Portal] or [PortalDestination] still exist
    /// 
    /// Defaults to despawn all entities and children with a warning, except for the main camera.
    /// Will be added as a Resource, can be changed during execution.
    pub despawn_strategy: PortalPartsDespawnStrategy,
}

impl Default for PortalsPlugin {
    fn default() -> Self {
        PortalsPlugin {
            check_create: PortalsCheckMode::AlwaysCheck,
            check_portal_camera_despawn: true,
            despawn_strategy: default(),
        }
    }
}

impl PortalsPlugin {
    pub const MINIMAL: Self = Self {
        check_create: PortalsCheckMode::CheckAfterStartup,
        check_portal_camera_despawn: false,
        despawn_strategy: PortalPartsDespawnStrategy::PANIC,
    };
}

impl Plugin for PortalsPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::load_internal_asset!(
            app,
            PORTAL_SHADER_HANDLE,
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/portal.wgsl"),
            Shader::from_wgsl
        );

        app
            .add_plugin(MaterialPlugin::<PortalMaterial>::default())
            .insert_resource(self.despawn_strategy)
            .register_type::<Portal>()
            .register_type::<PortalDestination>()
            .register_type::<PortalCamera>();

        app.add_system(update_portal_cameras.in_base_set(CoreSet::Last));

        if self.check_create != PortalsCheckMode::Manual {
            app.add_startup_system(create_portals.in_base_set(StartupSet::PostStartup).after(TransformSystem::TransformPropagate));
        }

        if self.check_create == PortalsCheckMode::AlwaysCheck {
            app.add_system(create_portals.in_base_set(CoreSet::PostUpdate).after(TransformSystem::TransformPropagate));
        }
        
        if self.check_portal_camera_despawn {
            app.add_system(check_portal_camera_despawn);
        }
    }
}

/// Whether and when [PortalsPlugin] should check for entities with [CreatePortal] components to create a portal using [create_portals].
#[derive(PartialEq, Eq)]
pub enum PortalsCheckMode {
    /// Don't set up this check automatically with the plugin, set-up [create_portals] manually, or use [CreatePortalCommand].
    Manual,
    /// Set up the check during [StartupSet::PostStartup], after [TransformSystem::TransformPropagate].
    CheckAfterStartup,
    /// Set up the check during [StartupSet::PostStartup] and [CoreSet::Last], after [TransformSystem::TransformPropagate].
    AlwaysCheck
}

/// Strategy to despawn portal parts.
/// 
/// Defaults to despawn all parts with a warning (without their children), except for the main camera.
#[derive(Resource, Clone, Copy)]
pub struct PortalPartsDespawnStrategy {
    pub main_camera: PortalPartDespawnStrategy,
    pub portal: PortalPartDespawnStrategy,
    pub destination: PortalPartDespawnStrategy,
    pub portal_camera: PortalPartDespawnStrategy,
}

impl Default for PortalPartsDespawnStrategy {
    fn default() -> Self {
        PortalPartsDespawnStrategy {
            main_camera: PortalPartDespawnStrategy::Leave,
            portal: default(),
            destination: default(),
            portal_camera: default(),
        }
    }
}

impl PortalPartsDespawnStrategy {
    pub const PANIC: Self = Self {
        main_camera: PortalPartDespawnStrategy::Leave,
        portal: PortalPartDespawnStrategy::Panic,
        destination: PortalPartDespawnStrategy::Panic,
        portal_camera: PortalPartDespawnStrategy::Panic,
    };

    pub const DESPAWN_SILENTLY: Self = Self {
        main_camera: PortalPartDespawnStrategy::Leave,
        portal: PortalPartDespawnStrategy::DespawnEntity,
        destination: PortalPartDespawnStrategy::DespawnEntity,
        portal_camera: PortalPartDespawnStrategy::DespawnEntity,
    };

    pub const DESPAWN_WITH_CHILDREN_SILENTLY: Self = Self {
        main_camera: PortalPartDespawnStrategy::Leave,
        portal: PortalPartDespawnStrategy::DespawnWithChildren,
        destination: PortalPartDespawnStrategy::DespawnWithChildren,
        portal_camera: PortalPartDespawnStrategy::DespawnWithChildren,
    };
}

/// Strategy to despawn a portal part if it is not yet despawned
#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum PortalPartDespawnStrategy {
    /// Despawn the entity and all of its children with a warning
    WarnThenDespawnWithChildren,
    /// Despawn the entity and all of its children
    DespawnWithChildren,
    /// Despawn only the entity with a warning
    #[default]
    WarnThenDespawnEntity,
    /// Despawn only the entity
    DespawnEntity,
    /// Don't despawn
    Leave,
    /// Panic
    Panic,
}

impl PortalPartDespawnStrategy {
    pub(super) fn should_panic(&self) -> bool {
        self == &Self::Panic
    }

    pub(super) fn should_despawn(&self) -> bool {
        self != &Self::Leave && self != &Self::Panic
    }

    pub(super) fn should_despawn_children(&self) -> bool {
        self == &Self::WarnThenDespawnWithChildren || self == &Self::DespawnWithChildren
    }

    pub(super) fn should_warn(&self) -> bool {
        self == &Self::WarnThenDespawnWithChildren || self == &Self::WarnThenDespawnEntity
    }
}

/// Bundle to create a portal with all the components needed.
#[derive(Bundle, Default)]
pub struct CreatePortalBundle {
    /// Mesh of the portal.
    pub mesh: Handle<Mesh>,
    /// Configuration of the portal.
    pub create_portal: CreatePortal,
    /// Transform of the portal.
    pub portal_transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

/// Component to create a [Portal] and everything needed to make it work.
/// 
/// The portal will be created after the next check (see [PortalsCheckMode]), if it has the other components in [CreatePortalBundle].
#[derive(Component, Clone)]
pub struct CreatePortal {
    /// Where the portal should lead to.
    pub destination: AsPortalDestination,
    /// The camera that will see this portal, defaults to the first camera found.
    pub main_camera: Option<Entity>,
    /// Whether to cull the “front”, “back” or neither side of a the portal mesh.
    /// 
    /// If set to None, the two sides of the portal are visible and work as a portal.
    /// 
    /// Defaults to Some(Face::Back), see [StandardMaterial].
    pub cull_mode: Option<Face>,
    /// Render layer used by the [PortalCamera], and debug elements.
    pub render_layer: RenderLayers,
    /// If Some(Face::Back), portal camera will get deactivated if camera is going behind the portal's transform.
    /// 
    /// Defaults to None temporarilly. 
    /// Some(Face::Front) deactivates the camera in front of the transform, and None never deactivates it.
    /// If your mesh isn't on a plane with cull_mode = Some(Face::Back), set this to None.
    pub plane_mode: Option<Face>,
    /// Configures debug elements, defaults to None.
    pub debug: Option<DebugPortal>,
}

impl Default for CreatePortal {
    fn default() -> Self {
        CreatePortal {
            destination: AsPortalDestination::Create(Default::default()),
            main_camera: None,
            cull_mode: Some(Face::Back),
            render_layer: Default::default(),
            plane_mode: None,
            debug: None,
        }
    }
}

/// How to create the [PortalDestination].
#[derive(Clone)]
pub enum AsPortalDestination {
    /// Use an already existing entity.
    Use(Entity),
    /// Create a [PortalDestination] with the given configuration.
    Create(CreatePortalDestination),
    /// Create a [PortalDestination] to make a mirror.
    /// 
    /// Will set the [PortalDestination] as a child of the [Portal] entity
    CreateMirror
}

/// [PortalDestination] to be created
#[derive(Clone, Default)]
pub struct CreatePortalDestination {
    /// Where to create the destination of the portal
    pub transform: Transform,
    ///Entity to use as a parent of the [PortalDestination]
    pub parent: Option<Entity>,
    //TODO: pub spawn_as_children: something like an EntityCommand?
}

/// Configuration of debug elements.
#[derive(Clone)]
pub struct DebugPortal {
    /// Name of the portal, used in the debug window's title.
    pub name: Option<String>,
    /// Color used by debug elements, defaults to gray.
    pub color: Color,
    /// If true, shows a debug window, it will use a copy of the [PortalCamera] marked with [PortalDebugCamera].
    pub show_window: bool,
    /// If true, displays a small sphere at the destination.
    pub show_destination_point: bool,
    /// If true, displays a copy of the portal mesh at the destination.
    pub show_portal_copy: bool,
    /// If true, displays a small sphere at the [PortalCamera] position.
    pub show_portal_camera_point: bool
}

impl Default for DebugPortal {
    fn default() -> Self {
        DebugPortal {
            name: Default::default(),
            color: Color::GRAY,
            show_window: true,
            show_destination_point: true,
            show_portal_copy: true,
            show_portal_camera_point: true
        }
    }
}