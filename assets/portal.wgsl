#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

struct FragmentInput {
    //@builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    /*if !in.is_front {
        return vec4(1.0, 1.0, 0.0, 1.0);
    }*/
    let dimensions = textureDimensions(texture);
    let dimension_x: f32 = f32(dimensions.x);
    let dimension_y: f32 = f32(dimensions.y);
    let uv: vec2<f32> = vec2(in.frag_coord.x/dimension_x, in.frag_coord.y/dimension_y);// / in.world_position.w;
    let color = textureSample(texture, texture_sampler, uv).rgb;
    return vec4(color, 1.0);
}