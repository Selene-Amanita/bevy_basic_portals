#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0)
var texture: texture_2d<f32>;
@group(2) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(texture);
    let dimension_x: f32 = f32(dimensions.x);
    let dimension_y: f32 = f32(dimensions.y);
    let uv: vec2<f32> = vec2(in.position.x/dimension_x, in.position.y/dimension_y);// / in.world_position.w;
    let color = textureSample(texture, texture_sampler, uv).rgb;
    return vec4(color, 1.0);
}