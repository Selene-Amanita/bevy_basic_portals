///! Projection logic for portals.

use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_reflect::{Reflect, std_traits::ReflectDefault};
use bevy_render::{
    prelude::*,
    camera::{CameraProjectionPlugin, CameraProjection},
};

/// Add the projection logic to [PortalsPlugin](super::PortalsPlugin)
pub(super) fn build_projection(app: &mut App) {
    app.add_plugin(CameraProjectionPlugin::<PortalProjection>::default());
}

/// For now, almost a copy of Bevy's Projection, to avoid frustum being calculated
/// from it automatically.
/// In the future, hopefully, will be used for Fitting projection.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub enum PortalProjection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
    //Fitting
}

impl Default for PortalProjection {
    fn default() -> Self {
        PortalProjection::Perspective(PerspectiveProjection::default())
    }
}

impl From<Projection> for PortalProjection {
    fn from(p: Projection) -> Self {
        match p {
            Projection::Perspective(projection) => Self::Perspective(projection),
            Projection::Orthographic(projection) => Self::Orthographic(projection),
        }
    }
}

impl From<PerspectiveProjection> for PortalProjection {
    fn from(p: PerspectiveProjection) -> Self {
        Self::Perspective(p)
    }
}

impl From<OrthographicProjection> for PortalProjection {
    fn from(p: OrthographicProjection) -> Self {
        Self::Orthographic(p)
    }
}

impl CameraProjection for PortalProjection {
    fn get_projection_matrix(&self) -> Mat4 {
        match self {
            Self::Perspective(projection) => projection.get_projection_matrix(),
            Self::Orthographic(projection) => projection.get_projection_matrix(),
        }
    }

    fn update(&mut self, width: f32, height: f32) {
        match self {
            Self::Perspective(projection) => projection.update(width, height),
            Self::Orthographic(projection) => projection.update(width, height),
        }
    }

    fn far(&self) -> f32 {
        match self {
            Self::Perspective(projection) => projection.far(),
            Self::Orthographic(projection) => projection.far(),
        }
    }
}