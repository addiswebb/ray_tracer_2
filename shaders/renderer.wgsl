struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

var<private> v_vertices: array<vec4<f32>, 4> = array<vec4<f32>, 4>(
    vec4<f32>(-1.0, -1.0, 1.0, 1.0),
    vec4<f32>( 1.0, -1.0, 1.0, 1.0),
    vec4<f32>( 1.0,  1.0, 1.0, 1.0),
    vec4<f32>(-1.0,  1.0, 1.0, 1.0),
);
var<private> v_texcoords: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 0.0),
    vec2<f32>(0.0,  1.0),
    vec2<f32>(1.0,  1.0),
);

var<private> v_indices: array<u32, 6> = array<u32, 6>(
    0,1,2,2,3,0
);

@vertex
fn vert(@builtin(vertex_index) i: u32) -> VertexOutput{
    var out: VertexOutput;
    out.position = v_vertices[v_indices[i]];
    out.tex_coord =v_texcoords[v_indices[i]];
    return out;
}

struct Params{
    width: u32,
    height: u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
    skybox: i32,
    frames: u32,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var texture: texture_2d<f32>;

@fragment
fn frag(i: VertexOutput) -> @location(0) vec4<f32>{
    var coords = vec2<i32>(
        i32(i.tex_coord.x * f32(params.width)),
        i32(i.tex_coord.y * f32(params.height))
    );
    var color = textureLoad(texture, coords, 0);
    return color;
}
