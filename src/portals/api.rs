//! Components and structs to create portals without caring about their implementation

use bevy_app::prelude::*;
use bevy_color::{palettes::basic::GRAY, Color};
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::{prelude::*, primitives::HalfSpace, render_resource::Face, view::RenderLayers};
use bevy_transform::prelude::*;

use super::*;

/// [Plugin] to add support for portals to a bevy App.
pub struct PortalsPlugin {
    /// If true, should check if any [PortalParts] entity despawned but still has a [PortalPart] referencing it with [check_portal_parts_back_references]
    pub check_portal_parts_back_references: bool,
    /// What to do when there is a problem getting a [PortalParts]
    ///
    /// Can happen when :
    /// - a part (main camera, [Portal], [PortalDestination]) has despawned but the [PortalCamera] still exists,
    /// - a part is missing a key component (see [CreatePortalParams], entities should be returned by the relevant queries).
    /// - check_portal_camera_despawn is true and a portal camera has despawned or missing a key component but the [Portal] or [PortalDestination] still exist
    ///
    /// Defaults/`None` to despawn all entities and children with a warning, except for the main camera.
    /// Will be added as a [Resource], can be changed during execution.
    pub despawn_strategy: Option<PortalPartsDespawnStrategy>,
}

impl Default for PortalsPlugin {
    fn default() -> Self {
        PortalsPlugin {
            check_portal_parts_back_references: true,
            despawn_strategy: None,
        }
    }
}

impl PortalsPlugin {
    pub const MINIMAL: Self = Self {
        check_portal_parts_back_references: false,
        despawn_strategy: Some(PortalPartsDespawnStrategy::PANIC),
    };
}

impl Plugin for PortalsPlugin {
    fn build(&self, app: &mut App) {
        build_material(app);
        build_projection(app);
        build_create(app);
        build_update(app);
        build_despawn(
            app,
            self.despawn_strategy.clone(),
            self.check_portal_parts_back_references,
        );

        #[cfg(feature = "picking_backend")]
        app.add_plugins(crate::picking::PortalPickingBackendPlugin);
    }
}

/// Strategy to despawn portal parts.
///
/// Defaults to despawn all parts with a warning (without their children), except for the main camera.
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct PortalPartsDespawnStrategy {
    pub main_camera: PortalPartDespawnStrategy,
    pub portal: PortalPartDespawnStrategy,
    pub destination: PortalPartDespawnStrategy,
    pub portal_camera: PortalPartDespawnStrategy,
}

impl Default for PortalPartsDespawnStrategy {
    fn default() -> Self {
        PortalPartsDespawnStrategy::DESPAWN_AND_WARN
    }
}

impl PortalPartsDespawnStrategy {
    pub const DESPAWN_AND_WARN: Self = Self {
        main_camera: PortalPartDespawnStrategy::Leave,
        portal: PortalPartDespawnStrategy::WarnThenDespawnEntity,
        destination: PortalPartDespawnStrategy::WarnThenDespawnEntity,
        portal_camera: PortalPartDespawnStrategy::WarnThenDespawnEntity,
    };

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
#[derive(Default, PartialEq, Eq, Copy, Clone, Reflect)]
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

/// [Component] to create a [Portal] and everything needed to make it work.
///
/// The portal will be created after the next check (see [PortalsCheckMode]), if it has the components required.
/// 
/// Requires [Mesh3d] to define the mesh of the portal, and all its dependencies. Indirectly requires [Transform] to locate the portal.
#[derive(Component, Clone)]
#[require(Mesh3d)]
pub struct CreatePortal {
    /// Where the portal should lead to.
    pub destination: PortalDestinationSource,
    /// What technique to use to render the portal effect, and how to define the
    /// frustum when applicable.
    pub portal_mode: PortalMode,
    /// The camera that will see this portal, defaults to the first camera found.
    pub main_camera: Option<Entity>,
    /// Whether to cull the “front”, “back” or neither side of a the portal mesh.
    ///
    /// If set to `None`, the two sides of the portal are visible and work as a portal.
    ///
    /// Defaults to `Some(Face::Back)`, see [StandardMaterial](bevy_pbr::StandardMaterial).
    pub cull_mode: Option<Face>,
    /// Render layer used by the [PortalCamera], and debug elements.
    pub render_layer: RenderLayers,
    /// Configures debug elements, defaults to None.
    pub debug: Option<DebugPortal>,
}

impl Default for CreatePortal {
    fn default() -> Self {
        Self {
            destination: PortalDestinationSource::Create(CreatePortalDestination::default()),
            portal_mode: PortalMode::default(),
            main_camera: None,
            cull_mode: Some(Face::Back),
            render_layer: RenderLayers::default(),
            debug: None,
        }
    }
}

/// How to create the [PortalDestination].
#[derive(Clone)]
pub enum PortalDestinationSource {
    /// Use an already existing entity.
    Use(Entity),
    /// Create a [PortalDestination] with the given configuration.
    Create(CreatePortalDestination),
    /// Create a [PortalDestination] to make a mirror.
    ///
    /// Will set the [PortalDestination] as a child of the [Portal] entity
    CreateMirror,
}

/// [PortalDestination] to be created
#[derive(Clone, Default)]
pub struct CreatePortalDestination {
    /// Where to create the destination of the portal
    pub transform: Transform,
    /// Entity to use as a parent of the [PortalDestination]
    pub parent: Option<Entity>,
    /// Mirror the image according to the destination's local x/right direction
    pub mirror_x: bool,
    /// Mirror the image according to the destination's local y/up direction
    pub mirror_y: bool,
    //TODO: pub spawn_as_children: something like an EntityCommand?
}

/// What technique to use to render the portal effect, and what entities are seen
/// or not through it.
#[derive(Clone)]
pub enum PortalMode {
    /// The portal effect will be rendered on a texture with the same size as
    /// the main camera's viewport, and a shader will define the UV-mapping using
    /// the portal viewed through the main camera as a mask.
    ///
    /// The frustum will simply be defined from the projection matrix, which means
    /// everything between the portal camera and the destination will be seen through
    /// the portal
    MaskedImageNoFrustum,
    /// Same as [PortalMode::MaskedImageNoFrustum], but a frustum will be defined,
    /// using a [HalfSpace] in the mesh/entity local space (it later takes into account
    /// the destination transform for calculations in global space).
    ///
    /// `None` will assume the `Plane` is `{p, p.z < 0}` in local space, it should
    /// be the same as `Some(Vec3::NEG_Z.extend(0.))`.
    ///
    /// Note that this will *replace* the near plane of the frustum defined from
    /// the projection matrix, which means that some objects might be considered
    /// for rendering when they shouldn't be (for example, when the camera's forward
    /// is almost parallel to the plane, objects behind the camera but in front of
    /// the plane will be considered).
    MaskedImageHalfSpaceFrustum(Option<HalfSpace>),
    //TODO
    //MaskedImageRectangleFrustum(PortalRectangleView),
    //MaskedImageSphereHalfSpaceFrustum(_)
    //MaskedImageSphereRectangleFrustum(_)
    // A projection matrix will be defined to fit.
    //FittingProjectionRectangle(PortalRectangleView)
}

impl Default for PortalMode {
    fn default() -> Self {
        PortalMode::MaskedImageHalfSpaceFrustum(None)
    }
}

/*#[derive(Clone)]
pub struct PortalRectangleView {
    origin: Vec3,
    normal: Vec3,
    rectangle: Vec2,
}*/

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
    pub show_portal_camera_point: bool,
}

impl Default for DebugPortal {
    fn default() -> Self {
        DebugPortal {
            name: Default::default(),
            color: GRAY.into(),
            show_window: true,
            show_destination_point: true,
            show_portal_copy: true,
            show_portal_camera_point: true,
        }
    }
}
