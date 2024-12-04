//! Projection logic for portals.

use bevy_app::{App, PostStartup, PostUpdate};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Vec3A};
use bevy_pbr::PbrProjectionPlugin;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{camera_system, CameraProjection, CameraUpdateSystem, SubCameraView},
    prelude::*,
};

/// Add the projection logic to [PortalsPlugin](super::PortalsPlugin)
pub(super) fn build_projection(app: &mut App) {
    // Copy of CameraProjectionPlugin's code but without update_frusta
    app.register_type::<PortalProjection>()
        .add_systems(
            PostStartup,
            camera_system::<PortalProjection>
                .in_set(CameraUpdateSystem)
                .ambiguous_with(CameraUpdateSystem),
        )
        .add_systems(
            PostUpdate,
            camera_system::<PortalProjection>
                .in_set(CameraUpdateSystem)
                .ambiguous_with(CameraUpdateSystem),
        );

    app.add_plugins(PbrProjectionPlugin::<PortalProjection>::default());
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
    fn get_clip_from_view(&self) -> Mat4 {
        match self {
            Self::Perspective(projection) => projection.get_clip_from_view(),
            Self::Orthographic(projection) => projection.get_clip_from_view(),
        }
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &SubCameraView) -> Mat4 {
        match self {
            Self::Perspective(projection) => projection.get_clip_from_view_for_sub(sub_view),
            Self::Orthographic(projection) => projection.get_clip_from_view_for_sub(sub_view),
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
