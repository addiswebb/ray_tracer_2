struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};
var<private> v_positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0, -1.0)
);

@vertex
fn vert(@builtin(vertex_index) v_idx: u32) -> VertexOutput{
    var out: VertexOutput;
    out.position = vec4<f32>(v_positions[v_idx], 0.0, 1.0);
    return out;
}

struct Params{
    width: u32,
    height: u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
    toggle: i32,
    frames: u32,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var texture: texture_2d<f32>;

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    var coords = vec2<i32>(
        i32(i.position.x * f32(params.width)),
        i32(i.position.y * f32(params.height))
    );
    var color = textureLoad(texture, coords, 0);
    return color;
}
