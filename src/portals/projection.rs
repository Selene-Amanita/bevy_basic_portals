//! Projection logic for portals.
//!
//! This is currently unused because it makes Bevy crash in some situations: https://github.com/bevyengine/bevy/issues/9077

use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Vec3A};
use bevy_reflect::{Reflect, std_traits::ReflectDefault};
use bevy_render::{
    prelude::*,
    camera::{CameraProjectionPlugin, CameraProjection},
};

/// Add the projection logic to [PortalsPlugin](super::PortalsPlugin)
pub(super) fn build_projection(app: &mut App) {
    app.add_plugins(CameraProjectionPlugin::<PortalProjection>::default());
}

/// For now, almost a copy of Bevy's Projection, to avoid frustum being calculated
/// from it automatically.
/// In the future, hopefully, will be used for Fitting projection.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub enum PortalProjection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
    //Other(Box<dyn CameraProjection>),
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

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        match self {
            Self::Perspective(projection) => projection.get_frustum_corners(z_near, z_far),
            Self::Orthographic(projection) => projection.get_frustum_corners(z_near, z_far),
        }
    }
}