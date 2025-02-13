#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0)
var texture: texture_2d<f32>;
@group(2) @binding(1)
var texture_sampler: sampler;
@group(2) @binding(2)
var<uniform> mirror_u: u32;
@group(2) @binding(3)
var<uniform> mirror_v: u32;

@fragment
fn fragment(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(texture);
    let dimension_x: f32 = f32(dimensions.x);
    let dimension_y: f32 = f32(dimensions.y);
    var uv: vec2<f32> = vec2(in.position.x/dimension_x, in.position.y/dimension_y);
    if (mirror_u != 0) { uv.x = 1. - uv.x; }
    if (mirror_v != 0) { uv.y = 1. - uv.y; }
    let color = textureSample(texture, texture_sampler, uv).rgb;
    return vec4(color, 1.0);
}