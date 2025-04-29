//! Material for portal rendering

use bevy_app::App;
use bevy_asset::{prelude::*, weak_handle};
use bevy_image::Image;
use bevy_pbr::prelude::*;
use bevy_pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy_reflect::TypePath;
use bevy_render::{
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
    },
};

/// Add the material logic to [PortalsPlugin](super::PortalsPlugin)
pub(super) fn build_material(app: &mut App) {
    bevy_asset::load_internal_asset!(
        app,
        PORTAL_SHADER_HANDLE,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/portal.wgsl"),
        Shader::from_wgsl
    );

    app.add_plugins(MaterialPlugin::<PortalMaterial>::default());
}

/// Material with the portal shader (renders the image without deformation using the mesh as a mask).
#[derive(Asset, AsBindGroup, Clone, TypePath)]
#[bind_group_data(PortalMaterialKey)]
pub struct PortalMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub mirror_u: u32,
    #[uniform(3)]
    pub mirror_v: u32,
    pub cull_mode: Option<Face>,
}

pub const PORTAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("1EA3049777A909BDFFEB794905C6D106");

impl Material for PortalMaterial {
    fn fragment_shader() -> ShaderRef {
        PORTAL_SHADER_HANDLE.into()
    }

    fn specialize(
        _: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PortalMaterialKey {
    cull_mode: Option<Face>,
}

impl From<&PortalMaterial> for PortalMaterialKey {
    fn from(material: &PortalMaterial) -> Self {
        PortalMaterialKey {
            cull_mode: material.cull_mode,
        }
    }
}
