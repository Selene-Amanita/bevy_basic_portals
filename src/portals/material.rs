///! Material for portal rendering

use bevy_app::App;
use bevy_asset::prelude::*;
use bevy_pbr::prelude::*;
use bevy_render::{
    prelude::*,
    mesh::MeshVertexBufferLayout,
    render_resource::{
        AsBindGroup,
        Face,
        RenderPipelineDescriptor,
        ShaderRef,
        SpecializedMeshPipelineError,
    },
};
use bevy_reflect::{TypeUuid, TypePath};
use bevy_pbr::{MaterialPipelineKey, MaterialPipeline};

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
#[derive(AsBindGroup, Clone, TypeUuid, TypePath)]
#[bind_group_data(PortalMaterialKey)]
#[uuid = "436e9734-867f-4faf-9b5f-81703017a018"]
pub struct PortalMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    pub cull_mode: Option<Face>
}

pub const PORTAL_SHADER_HANDLE: HandleUntyped =
  HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x792531383ac40e25);

impl Material for PortalMaterial {
    fn fragment_shader() -> ShaderRef {
        PORTAL_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        _: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayout,
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