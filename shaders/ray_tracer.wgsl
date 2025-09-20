struct Params{
    width: u32,
    height: u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
    toggle: i32,
    frames: i32,
};
struct Material{
    color: vec4<f32>,
    emission_color: vec4<f32>,
    emission_strength: f32,
    smoothness: f32,
}

struct Sphere{
    position: vec3<f32>,
    radius: f32,
    material: Material,
};

struct Vertex{
    pos: vec3<f32>,
    normal: vec3<f32>
};

struct Mesh{
    first: u32,
    triangles: u32,
    offset: u32,
    pos: vec3<f32>,
    material: Material,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var<uniform> camera: Camera;
@group(0) @binding(2)
var texture: texture_storage_2d<rgba32float,read_write>;
@group(0) @binding(3)
var<storage,read> spheres: array<Sphere>;
@group(0) @binding(4)
var<storage,read> vertices: array<Vertex>;
@group(0) @binding(5)
var<storage,read> indices: array<u32>;
@group(0) @binding(6)
var<storage,read> meshes: array<Mesh>;

const epsilon: f32 = 1e-4;

@compute
@workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i: FragInput;

    i.pos = vec2<f32>(f32(global_id.x),f32(global_id.y));
    i.size = vec2<f32>(f32(params.width),f32(params.height));

    let pos = vec2<i32>(i32(i.pos.x),i32(i.pos.y));
    if(params.frames >= 1){
        let weight = 1.0 / f32(params.frames);
        let prev_color = textureLoad(texture,pos);
        let new_color = prev_color * vec4<f32>(1.0 - weight) + frag(i) * weight;
        textureStore(texture, pos, new_color);
    }else{
        textureStore(texture, pos, frag(i));
    }
}

struct Camera{
    origin: vec3<f32>,
    lower_left_corner: vec3<f32>,
    horizontal: vec3<f32>,
    vertical: vec3<f32>,
    near: f32,
    far: f32,
    w: vec3<f32>,
    u: vec3<f32>,
    v: vec3<f32>,
    lens_radius: f32,
}

struct FragInput{
    pos: vec2<f32>,
    size: vec2<f32>,
};

struct Ray{
    origin: vec3<f32>,
    dir: vec3<f32>,
}

struct Hit{
    hit: bool,
    dst: f32,
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    material: Material,
}

const SKY_HORIZON: vec4<f32> = vec4<f32>(1.0,1.0,1.0,0.0);
const SKY_ZENITH: vec4<f32> = vec4<f32>(0.0788092, 0.36480793, 0.7264151, 0.0);
const GROUND_COLOR: vec4<f32> = vec4<f32>(0.35,0.3,0.35, 0.0);
const SUN_INTENSITY: f32 = 0.1;
const SUN_FOCUS: f32 = 500.0;

fn safe_normalize(vec: vec3<f32>) -> vec3<f32>{
    var n = normalize(vec);
    if (n.x != n.x || n.y != n.y || n.z != n.z){
        n = vec3<f32>(0.0,1.0,0.0);
    }
    return n;
}

fn ray_sphere(ray: Ray, pos: vec3<f32>, radius: f32) -> Hit{
    var hit: Hit;
    let oc = ray.origin - pos;
    let a = dot(ray.dir,ray.dir);
    let b = 2.0 * dot(oc, ray.dir);
    let c = dot(oc,oc) - pow(radius,2.0);
    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0{
        let dst = (-b - sqrt(discriminant))/(2.0*a);
        if dst > epsilon{
            hit.hit = true;
            hit.hit_point = ray.origin + ray.dir * dst;
            hit.dst = dst;
            hit.normal = safe_normalize(hit.hit_point - pos);
        }
    }
    return hit;
}

fn ray_triangle(ray: Ray, a: Vertex, b: Vertex, c: Vertex) -> Hit{
    var hit: Hit;
    let edge_ab = b.pos - a.pos;
    let edge_ac = c.pos - a.pos;
    let normal = cross(edge_ab, edge_ac);
    let ao = ray.origin - a.pos;
    let dao = cross(ao, ray.dir);

    let determinant = -dot(ray.dir, normal);
    if determinant < epsilon {
        hit.hit = false;
        return hit;
    }
    let inverse_determinant = 1.0 / determinant;

    let dst = dot(ao, normal) * inverse_determinant;
    let u = dot(edge_ac, dao) * inverse_determinant;
    let v = -dot(edge_ab, dao) * inverse_determinant;
    let w = 1.0 - u - v;

    if  dst > epsilon && u >= 0.0 && v >= 0.0 && w >= 0.0 {
        hit.hit = true;
        hit.hit_point = ray.origin + ray.dir * dst;
        hit.normal = safe_normalize(a.normal * w + b.normal * u + c.normal * v);
        hit.dst = dst;
    }else{
        hit.hit = false;
    }

    return hit;
}

fn calculate_ray_collions(ray: Ray) -> Hit{
    var closest_hit: Hit;
    closest_hit.dst = 0x1.fffffep+127f;
    for(var i: u32 = 0u; i < arrayLength(&spheres); i+=1u){
        var hit: Hit = ray_sphere(ray, spheres[i].position, spheres[i].radius);
        if hit.hit && hit.dst < closest_hit.dst{
            closest_hit = hit;
            closest_hit.material = spheres[i].material;
        }
    }
    for(var mesh_index: u32 = 0u; mesh_index< arrayLength(&meshes); mesh_index+=1u){
        for(var i: u32 = 0u; i < meshes[mesh_index].triangles; i+=1u){
            let first = meshes[mesh_index].first;
            let offset = meshes[mesh_index].offset;

            let index1 = indices[first + i*3u];
            let index2 = indices[first + i*3u+1u];
            let index3 = indices[first + i*3u+2u];

            var v1 = vertices[offset + index1];
            var v2 = vertices[offset + index2];
            var v3 = vertices[offset + index3];

            v1.pos += meshes[mesh_index].pos;
            v2.pos += meshes[mesh_index].pos;
            v3.pos += meshes[mesh_index].pos;

            var hit: Hit = ray_triangle(ray, v1,v2,v3);
            if hit.hit && hit.dst < closest_hit.dst{
                closest_hit = hit;
                closest_hit.material = meshes[mesh_index].material;
            }
        }
    }

    return closest_hit;
}

fn rand(seed: ptr<function,u32>) -> f32 {
    return f32(next_random_number(seed)) / 4294967295.0; // 2^32 - 1
}

fn rand_unit_sphere(seed: ptr<function, u32>) -> vec3<f32> {
    let x = rand_normal_dist(seed);
    let y = rand_normal_dist(seed);
    let z = rand_normal_dist(seed);

    return safe_normalize(vec3(x, y, z));
}

fn rand_normal_dist(seed: ptr<function, u32>) -> f32 {
    let theta = 2.0 * 3.1415926 * rand(seed);
    let rho = sqrt(-2.0 * log(rand(seed)));
    return rho * cos(theta);
}

fn next_random_number(seed: ptr<function,u32>) -> u32 {
    *seed = *seed * 747796405u + 2891336453u;
    var result: u32 = ((*seed >> ((*seed >> 28u) + 4u)) ^ *seed) * 277803737u;
    result = (result >> 22u) ^ result;
    return result;
}
fn rand_hemisphere_dir_dist(normal: vec3<f32>, seed: ptr<function, u32>) -> vec3<f32>{
    let dir = rand_unit_sphere(seed);
    return dir * sign(dot(normal, dir));
}

fn rand_in_unit_disk(seed: ptr<function, u32>)-> vec3<f32>{
    for(var i = 0; i < 1000; i+=1){
        let r1 = (rand(seed)*2.0);
        let r2 = (rand(seed)*2.0);
        let p = vec3<f32>(r1, r2, 1.0) - 1.0;
        if(length(p) <= 1.0){
            return p;
        }
        i -=1;
    }
    return vec3<f32>(0.0,0.0,0.0);
}

fn trace(incident_ray: Ray, seed: ptr<function, u32>) -> vec4<f32>{
    var ray: Ray = incident_ray;
    var ray_color = vec4<f32>(1.0);
    var incoming_light = vec4<f32>(0.0);
    for (var i = 0; i <= params.number_of_bounces; i +=1){
        var hit = calculate_ray_collions(ray);
        if (hit.hit){
            ray.origin = hit.hit_point;
            let unit_dir = safe_normalize(ray.dir);
            //TODO look into different random hemisphere generators
            //-1.0 is for glass
            if(hit.material.smoothness < 0.0){
                hit.material.color = vec4<f32>(1.0);

                var front_face = false;
                if(dot(ray.dir, hit.normal) > 0.0){
                    front_face = false;
                }else{
                    front_face = true;
                }
                //TODO get refraction_ratio from material
                // var refraction_ratio = 1.5;
                var refraction_ratio = -hit.material.smoothness;
                if(front_face){
                    refraction_ratio = 1.0/1.5;
                }

                // let cos_theta = min(dot(-unit_dir, hit.normal),1.0);
                let cos_theta = clamp(dot(-unit_dir, hit.normal), -1.0, 1.0);
                let sin_theta = sqrt(1.0 - cos_theta*cos_theta);

                let cannot_refract = refraction_ratio * sin_theta > 1.0;

                if(cannot_refract || reflectance(cos_theta,refraction_ratio) > rand(seed)){
                    ray.dir = reflect(unit_dir, hit.normal);
                }else{
                    ray.dir = refract(unit_dir,hit.normal,refraction_ratio);
                }
            } else{
                var diffuse_dir = rand_hemisphere_dir_dist(hit.normal, seed);
                let specular_dir = reflect(unit_dir, hit.normal);
                ray.dir = mix(diffuse_dir,specular_dir,hit.material.smoothness);
            }

            let emitted_light = hit.material.emission_color * hit.material.emission_strength;
            incoming_light += emitted_light * ray_color;
            ray_color *= hit.material.color;
        }else{
            if(params.toggle != 0){
                incoming_light += get_environment_light(ray) * ray_color;
            }
            break;
        }
    }
    return incoming_light;
}

fn reflectance(cosine: f32, refraction_ratio: f32) -> f32{
    var r0 = (1.0 - refraction_ratio) / (1.0 + refraction_ratio);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow((1.0-cosine),5.0);
}

fn refract(uv: vec3<f32>, normal: vec3<f32>, refraction_ratio: f32) -> vec3<f32>{
    let cos_theta = min(dot(-uv, normal),1.0);
    let r_out_perp = refraction_ratio * (uv + cos_theta * normal);
    let r_out_parallel = -sqrt(abs(1.0 - length(r_out_perp))) * normal;
    return r_out_perp + r_out_parallel;
}

fn get_environment_light(ray: Ray) -> vec4<f32>{
    let sky_gradient_t = pow(smoothstep(0.0, 0.4, ray.dir.y), 0.35);
    let ground_to_sky_t = smoothstep(-0.01, 0.0, ray.dir.y);
    let sky_gradient = mix(SKY_HORIZON,SKY_ZENITH, sky_gradient_t);
    let sun = pow(max(0.0, dot(ray.dir, vec3<f32>(0.1,1.0,0.1))),SUN_FOCUS) * SUN_INTENSITY;
    let composite = mix(GROUND_COLOR, sky_gradient,ground_to_sky_t) + sun * f32(ground_to_sky_t >=1.0);
    return composite;
}

fn frag(i: FragInput) -> vec4<f32>{
    let pixel_coord = i.pos * i.size;
    var rng_state = u32(pixel_coord.y * i.size.x + pixel_coord.x)+u32(abs(params.frames)) * 71939u;

    var total_incoming_light = vec4<f32>(0.0);

    for (var j = 0; j <= params.rays_per_pixel; j+=1){
        let anti_aliasing = vec2<f32>(rand(&rng_state),rand(&rng_state));
        let pos = (i.pos + anti_aliasing) / i.size;

        let rd = camera.lens_radius * rand_in_unit_disk(&rng_state);
        let offset = camera.u * rd.x + camera.v * rd.y;

        var ray: Ray;
        ray.origin = camera.origin + offset;
        ray.dir = camera.lower_left_corner + pos.x * camera.horizontal + pos.y * camera.vertical - ray.origin;

        total_incoming_light += trace(ray, &rng_state);
    }
    let color = total_incoming_light/f32(params.rays_per_pixel);
    if (color.x != color.x || color.y != color.y || color.z != color.z){
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    return color;
}
