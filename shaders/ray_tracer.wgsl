struct Params {
    width: u32,
    height: u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
    skybox: i32,
    frames: i32,
    accumulate: i32,
    debug_flag: i32,
    debug_scale: i32,
};
struct Material {
    color: vec4<f32>,
    emission_color: vec4<f32>,
    specular_color: vec4<f32>,
    emission_strength: f32,
    smoothness: f32,
    specular: f32,
}

struct Sphere {
    position: vec3<f32>,
    radius: f32,
    material: Material,
};

struct Vertex {
    pos: vec3<f32>,
    normal: vec3<f32>
};

struct Mesh {
    first: u32,
    triangles: u32,
    offset: u32,
    pos: vec3<f32>,
    material: Material,
};
struct Scene {
    spheres: u32,
    vertices: u32,
    indices: u32,
    meshes: u32,
    camera: Camera,
    n_nodes: u32,
}

struct BVHNode {
    aabb_min: vec3<f32>,
    pad0: f32,
    aabb_max: vec3<f32>,
    pad1: f32,
    left: u32,
    right: u32,
    first: u32,
    count: u32,
};


struct Camera {
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

struct FragInput {
    pos: vec2<f32>,
    size: vec2<f32>,
};

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
    inv_dir: vec3<f32>,
}

struct Hit {
    hit: bool,
    dst: f32,
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    material: Material,
}

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var<uniform> scene: Scene;
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
@group(0) @binding(7)
var<storage,read> nodes: array<BVHNode>;
@group(0) @binding(8)
var<storage,read> tri_idx: array<u32>;

const epsilon: f32 = 1e-5;

@compute
@workgroup_size(16,16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i: FragInput;

    i.pos = vec2<f32>(f32(global_id.x), f32(global_id.y));
    i.size = vec2<f32>(f32(params.width), f32(params.height));

    let pos = vec2<i32>(i32(i.pos.x), i32(i.pos.y));
    let current_sample = frag(i);
    if params.frames >= 1 {
        let prev_color = textureLoad(texture, pos);
        let weight = 1.0 / f32(params.frames + 1);
        let new_color = prev_color * (1.0 - weight) + current_sample * weight;
        textureStore(texture, pos, new_color);
    } else {
        textureStore(texture, pos, current_sample);
    }
}

const SKY_HORIZON: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 0.0);
const SKY_ZENITH: vec4<f32> = vec4<f32>(0.0788092, 0.36480793, 0.7264151, 0.0);
const GROUND_COLOR: vec4<f32> = vec4<f32>(0.35, 0.3, 0.35, 0.0);
const SUN_INTENSITY: f32 = 0.1;
const SUN_FOCUS: f32 = 500.0;

fn safe_normalize(vec: vec3<f32>) -> vec3<f32> {
    var n = normalize(vec);
    if n.x != n.x || n.y != n.y || n.z != n.z {
        n = vec3<f32>(0.0, 1.0, 0.0);
    }
    return n;
}

fn ray_sphere(ray: Ray, centre: vec3<f32>, radius: f32) -> Hit {
    var hit: Hit;
    hit.hit = false;
    let offest_ray_origin = ray.origin - centre;
    // ax^2+bx+c=0
    // Represents a quadratic from sqrt(ray.origin + ray.dir * ray.dst) = radius^2
    let a = dot(ray.dir, ray.dir);
    let b = 2.0 * dot(offest_ray_origin, ray.dir);
    let c = dot(offest_ray_origin, offest_ray_origin) - pow(radius, 2.0);
    let discriminant = b * b - 4.0 * a * c;
    // No solution to quadratic
    if discriminant >= 0.0 {
        // Solve quadratic for distance
        let dst = (-b - sqrt(discriminant)) / (2.0 * a);
        if dst >= epsilon {
            hit.hit = true;
            hit.hit_point = ray.origin + ray.dir * dst;
            hit.dst = dst;
            hit.normal = safe_normalize(hit.hit_point - centre);
        }
    }
    return hit;
}

fn ray_triangle(ray: Ray, a: Vertex, b: Vertex, c: Vertex) -> Hit {
    var hit: Hit;
    let edge_ab = b.pos - a.pos;
    let edge_ac = c.pos - a.pos;
    let normal = cross(edge_ab, edge_ac);
    let ao = ray.origin - a.pos;
    let dao = cross(ao, ray.dir);

    let determinant = -dot(ray.dir, normal);
    if determinant < 0.000001 {
        hit.hit = false;
        return hit;
    }
    let inverse_determinant = 1.0 / determinant;

    let dst = dot(ao, normal) * inverse_determinant;
    let u = dot(edge_ac, dao) * inverse_determinant;
    let v = -dot(edge_ab, dao) * inverse_determinant;
    let w = 1.0 - u - v;

    if dst > epsilon && u >= 0.0 && v >= 0.0 && w >= 0.0 {
        hit.hit = true;
        hit.hit_point = ray.origin + ray.dir * dst;
        hit.normal = safe_normalize(a.normal * w + b.normal * u + c.normal * v);
        hit.dst = dst;
    } else {
        hit.hit = false;
    }

    return hit;
}

fn ray_BVH(ray: Ray, stats: ptr<function, vec2<i32>>) -> Hit {
    var closest_hit: Hit;
    closest_hit.hit = false;
    closest_hit.dst = 1e30;
    var stack: array<u32,64>;
    var stack_index: u32 = 0u;
    stack[stack_index] = 0u;
    stack_index += 1u;

    while stack_index > 0u {
        stack_index -= 1u;
        let node = nodes[stack[stack_index]];
        // Is Leaf node?
        if node.count > 0u {
            (*stats)[1] += i32(node.count); // Track triangle checks
            for (var j: u32 = 0u; j < node.count; j += 1u) {
                let tri_num = tri_idx[node.first + j];
                let mesh_index = get_mesh(tri_num);

                let index1 = indices[tri_num * 3u];
                let index2 = indices[tri_num * 3u + 1u];
                let index3 = indices[tri_num * 3u + 2u];

                var v1 = vertices[index1];
                var v2 = vertices[index2];
                var v3 = vertices[index3];
                let hit = ray_triangle(ray, v1, v2, v3);
                if hit.hit && hit.dst < closest_hit.dst {
                    closest_hit = hit;
                    closest_hit.material = meshes[mesh_index].material;
                }
            }
        } else { // Otherwise its root node, push children onto the stack
            let left_node = nodes[node.left];
            let right_node = nodes[node.right];
            let left_dst = ray_aabb_dist(ray, left_node.aabb_min, left_node.aabb_max, closest_hit.dst);
            let right_dst = ray_aabb_dist(ray, right_node.aabb_min, right_node.aabb_max, closest_hit.dst);
            (*stats)[0] += 2; // Track bounding box checks
            // Use index math to simplify code here:
            let left_is_closer = left_dst < right_dst;
            let near_dst = select(right_dst, left_dst, left_is_closer);
            let far_dst = select(right_dst, left_dst, !left_is_closer);
            let near_idx = select(node.right, node.left, left_is_closer);
            let far_idx = select(node.right, node.left, !left_is_closer);
            // Push farthest child first, (last on first off, last child gets checked first)
            if far_dst < closest_hit.dst { stack[stack_index] = far_idx; stack_index += 1u; }
            if near_dst < closest_hit.dst { stack[stack_index] = near_idx; stack_index += 1u; }
        }
    }
    return closest_hit;
}

fn get_mesh(triangle_index: u32) -> i32 {
    var mesh_index = -1;
    for (var m: u32 = 0u; m < scene.meshes; m += 1u) {
        let mesh: Mesh = meshes[m];
        let first_tri_index = mesh.first / 3u;
        if triangle_index >= first_tri_index && triangle_index < first_tri_index + mesh.triangles {
            mesh_index = i32(m);
            break;
        }
    }
    return mesh_index;
}

fn ray_aabb_dist(ray: Ray, b_min: vec3<f32>, b_max: vec3<f32>, t: f32) -> f32 {
    let tx1 = (b_min.x - ray.origin.x) * ray.inv_dir.x;
    let tx2 = (b_max.x - ray.origin.x) * ray.inv_dir.x;
    var tmin = min(tx1, tx2);
    var tmax = max(tx1, tx2);
    let ty1 = (b_min.y - ray.origin.y) * ray.inv_dir.y;
    let ty2 = (b_max.y - ray.origin.y) * ray.inv_dir.y;
    tmin = max(tmin, min(ty1, ty2));
    tmax = min(tmax, max(ty1, ty2));
    let tz1 = (b_min.z - ray.origin.z) * ray.inv_dir.z;
    let tz2 = (b_max.z - ray.origin.z) * ray.inv_dir.z;
    tmin = max(tmin, min(tz1, tz2));
    tmax = min(tmax, max(tz1, tz2));
    let did_hit = tmax >= tmin && tmin < t && tmax > 0.0;
    if did_hit {
        return tmin;
    } else {
        return 0x1.fffffep+127f;
    }
}

fn calculate_ray_collions(ray: Ray, stats: ptr<function, vec2<i32>>) -> Hit {
    var closest_hit: Hit;
    closest_hit.hit = false;
    closest_hit.dst = 0x1.fffffep+127f;

    for (var i: u32 = 0u; i < scene.spheres; i += 1u) {
        let hit: Hit = ray_sphere(ray, spheres[i].position, spheres[i].radius);
        if hit.hit && hit.dst < closest_hit.dst {
            closest_hit = hit;
            closest_hit.material = spheres[i].material;
        }
    }
    if scene.n_nodes > 0u {
        let hit: Hit = ray_BVH(ray, stats);
        if hit.hit && hit.dst < closest_hit.dst {
            closest_hit = hit;
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
fn rand_hemisphere_dir_dist(normal: vec3<f32>, seed: ptr<function, u32>) -> vec3<f32> {
    let dir = rand_unit_sphere(seed);
    return dir * sign(dot(normal, dir));
}

fn rand_in_unit_disk(seed: ptr<function, u32>) -> vec3<f32> {
    for (var i = 0; i < 1000; i += 1) {
        let r1 = (rand(seed) * 2.0);
        let r2 = (rand(seed) * 2.0);
        let p = vec3<f32>(r1, r2, 1.0) - 1.0;
        if length(p) <= 1.0 {
            return p;
        }
        i -= 1;
    }
    return vec3<f32>(0.0, 0.0, 0.0);
}

fn trace(incident_ray: Ray, seed: ptr<function, u32>) -> vec4<f32> {
    var ray: Ray = incident_ray;
    var ray_color = vec4<f32>(1.0);
    var incoming_light = vec4<f32>(0.0);
    var _stats = vec2<i32>(0, 0);
    for (var i = 0; i <= params.number_of_bounces; i += 1) {
        var hit = calculate_ray_collions(ray, &_stats);
        if hit.hit {
            let is_specular_bounce = hit.material.specular >= rand(seed);
            ray.origin = hit.hit_point;
            let unit_ray_dir = safe_normalize(ray.dir);
            // Used for glass
            if hit.material.smoothness < 0.0 {
                var front_face = dot(unit_ray_dir, hit.normal) < 0.0;
                var refractive_index = -hit.material.smoothness;
                // TODO, select(if false, if true, condition) i think this is backwards?
                refractive_index = select(refractive_index, 1.0 / refractive_index, front_face);

                let cos_theta = clamp(dot(-unit_ray_dir, hit.normal), -1.0, 1.0);
                let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
                let cannot_refract = refractive_index * sin_theta > 1.0;
                if cannot_refract || reflectance(cos_theta, refractive_index) > rand(seed) {
                    ray.dir = reflect(unit_ray_dir, hit.normal);
                } else {
                    ray.dir = refract(unit_ray_dir, hit.normal, refractive_index);
                }
            } else {
                let diffuse_dir = normalize(hit.normal + rand_unit_sphere(seed));
                let specular_dir = reflect(unit_ray_dir, hit.normal);
                ray.dir = normalize(mix(diffuse_dir, specular_dir, hit.material.smoothness * f32(is_specular_bounce)));
            }

            let emitted_light = hit.material.emission_color * hit.material.emission_strength;
            incoming_light += emitted_light * ray_color;
            if is_specular_bounce {
                ray_color *= hit.material.specular_color;
            } else {
                ray_color *= hit.material.color;
            }
            let p = max(ray_color.x, max(ray_color.y, ray_color.z));
            if rand(seed) >= p {
                break;
            }
            ray_color *= 1.0 / p;
        } else {
            // Use get_environment_light if skybox is enabled
            if params.skybox != 0 {
                incoming_light += get_environment_light(ray) * ray_color;
            }
            break;
        }
        ray.inv_dir = 1.0 / ray.dir;
    }

    return incoming_light;
}

fn reflectance(cosine: f32, refraction_ratio: f32) -> f32 {
    var r0 = (1.0 - refraction_ratio) / (1.0 + refraction_ratio);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow((1.0 - cosine), 5.0);
}

fn refract(uv: vec3<f32>, normal: vec3<f32>, refraction_ratio: f32) -> vec3<f32> {
    let cos_theta = min(dot(-uv, normal), 1.0);
    let r_out_perp = refraction_ratio * (uv + cos_theta * normal);
    let r_out_parallel = -sqrt(abs(1.0 - length(r_out_perp))) * normal;
    return r_out_perp + r_out_parallel;
}

fn get_environment_light(ray: Ray) -> vec4<f32> {
    let sky_gradient_t = pow(smoothstep(0.0, 0.4, ray.dir.y), 0.35);
    let ground_to_sky_t = smoothstep(-0.01, 0.0, ray.dir.y);
    let sky_gradient = mix(SKY_HORIZON, SKY_ZENITH, sky_gradient_t);
    let sun = pow(max(0.0, dot(ray.dir, vec3<f32>(0.1, 1.0, 0.1))), SUN_FOCUS) * SUN_INTENSITY;
    let composite = mix(GROUND_COLOR, sky_gradient, ground_to_sky_t) + sun * f32(ground_to_sky_t >= 1.0);
    return composite;
}

fn frag(i: FragInput) -> vec4<f32> {
    let pixel_coord = i.pos * i.size;
    var rng_state = u32(pixel_coord.y * i.size.x + pixel_coord.x) + u32(abs(params.frames)) * 719393u;
    if params.debug_flag != 0 {
        return debug_trace(i);
    }
    var total_incoming_light = vec4<f32>(0.0);
    for (var j = 0; j <= params.rays_per_pixel; j += 1) {
        let anti_aliasing = vec2<f32>(rand(&rng_state), rand(&rng_state));
        let pos = (i.pos + anti_aliasing) / i.size;

        let rd = scene.camera.lens_radius * rand_in_unit_disk(&rng_state);
        let offset = scene.camera.u * rd.x + scene.camera.v * rd.y;

        var ray: Ray;
        ray.origin = scene.camera.origin + offset;
        ray.dir = scene.camera.lower_left_corner + pos.x * scene.camera.horizontal + pos.y * scene.camera.vertical - ray.origin;
        ray.inv_dir = 1.0 / ray.dir;

        total_incoming_light += trace(ray, &rng_state);
    }
    let color = total_incoming_light / f32(params.rays_per_pixel);
    return color;
}

fn debug_trace(i: FragInput) -> vec4<f32> {
    var stats = vec2<i32>(0, 0);
    var ray: Ray;
    let pos = i.pos / i.size;
    ray.origin = scene.camera.origin ;
    ray.dir = scene.camera.lower_left_corner + pos.x * scene.camera.horizontal + pos.y * scene.camera.vertical - ray.origin;
    ray.inv_dir = 1.0 / ray.dir;
    let hit: Hit = calculate_ray_collions(ray, &stats);
    switch params.debug_flag{
        case 1: {
            let d = f32(stats[0]) / f32(params.debug_scale);
            if d > 1.0 {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            }
            return vec4<f32>(d, d, d, 1.0);
        }
        case 2:{
            let t = f32(stats[1]) / f32(params.debug_scale);
            if t > 1.0 {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            }
            return vec4<f32>(t, t, t, 1.0);
        }
        case 3:{
            let d = distance(ray.origin, hit.hit_point) / f32(params.debug_scale);
            return vec4<f32>(d, d, d, 1.0);
        }
        case 4:{
            if !hit.hit {return vec4<f32>(0.0);}
            let n = hit.normal * 0.5 + 0.5;
            return vec4<f32>(n.x, n.y, n.z, 1.0);
        }
        case 5:{
            let d = f32(stats[0]) / f32(params.debug_scale);
            let t = f32(stats[1]) / f32(params.debug_scale);
            return vec4<f32>(t, 0.0, d, 1.0);
        }
        case 6: {
            if !hit.hit {return vec4<f32>(0.0); }
            let d = hit.dst / f32(params.debug_scale);
            return vec4<f32>(d, d, d, 1.0);
        }
        default: {
            return vec4<f32>(1.0, 0.0, 1.0, 1.0);
        }
    }
}
