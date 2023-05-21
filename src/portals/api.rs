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
    pub check_create: PortalsCheckMode
}

impl Default for PortalsPlugin {
    fn default() -> Self {
        PortalsPlugin {
            check_create: PortalsCheckMode::AlwaysCheck
        }
    }
}

impl Plugin for PortalsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(MaterialPlugin::<PortalMaterial>::default())
            .add_system(update_portal_cameras.in_base_set(CoreSet::Last));

        if self.check_create != PortalsCheckMode::Manual {
            app.add_startup_system(create_portals.in_base_set(StartupSet::PostStartup).after(TransformSystem::TransformPropagate));
        }

        if self.check_create == PortalsCheckMode::AlwaysCheck {
            app.add_system(create_portals.in_base_set(CoreSet::PostUpdate).after(TransformSystem::TransformPropagate));
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

/// Component to create a portal, containing the informations needed.
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

/// How to create the portal destination
#[derive(Clone)]
pub enum AsPortalDestination {
    /// Use an already existing entity
    Use(Entity),
    /// Create a portal destination with the given configuration
    Create(CreatePortalDestination),
    /// Create a portal destination to make a mirror
    /// 
    /// Warning: this uses the portal's [GlobalTransform] to infer the destination's [Transform].
    /// Make sure that this GlobalTransform is set correctly when the portal is created.
    /// This shouldn't be a problem if you are not using [PortalsCheckMode::Manual]
    /// Warning: if the portal moves, the destination won't be updated
    // TO FIX
    CreateMirror
}

/// Portal destination to be created
#[derive(Clone, Default)]
pub struct CreatePortalDestination {
    /// Where to create the destination of the portal
    pub transform: Transform,
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