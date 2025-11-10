const INF: f32 = 0x1p+127f;  // Hexadecimal float literal
const MATERIAL_GLASS: i32 = 1;

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
}

struct Material {
    color: vec4<f32>,
    emission_color: vec4<f32>,
    specular_color: vec4<f32>,
    absorption: vec4<f32>,
    absorption_strength: f32,
    emission_strength: f32,
    smoothness: f32,
    specular: f32,
    ior: f32,
    flag: i32,
}

struct Sphere {
    position: vec3<f32>,
    radius: f32,
    material: Material,
}

struct Mesh {
    world_to_model: mat4x4<f32>,
    model_to_world: mat4x4<f32>,
    node_offset: u32,
    triangles: u32,
    triangle_offset: u32,
    material: Material,
}

struct Camera {
    origin: vec3<f32>,
    lens_radius: f32,
    lower_left_corner: vec3<f32>,
    near: f32,
    horizontal: vec3<f32>,
    far: f32,
    vertical: vec3<f32>,
    w: vec3<f32>,
    u: vec3<f32>,
    v: vec3<f32>,
}
struct Scene {
    spheres: u32,
    vertices: u32,
    indices: u32,
    meshes: u32,
    camera: Camera,
    n_nodes: u32,
}

struct BVHNode {
    left: u32,
    right: u32,
    first: u32,
    count: u32,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
}

struct Triangle {
    v1: vec3<f32>,
    v2: vec3<f32>,
    v3: vec3<f32>,
    n1: vec3<f32>,
    n2: vec3<f32>,
    n3: vec3<f32>,
}

struct FragInput {
    pos: vec2<f32>,
    size: vec2<f32>,
}

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
    inv_dir: vec3<f32>,
    transmittance: vec4<f32>,
    bounces: u32,
}

struct Hit {
    hit: bool,
    dst: f32,
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    backface: bool,
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
var<storage,read> triangles: array<Triangle>;
@group(0) @binding(5)
var<storage,read> meshes: array<Mesh>;
@group(0) @binding(6)
var<storage,read> nodes: array<BVHNode>;

const SKY_HORIZON: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 0.0);
const SKY_ZENITH: vec4<f32> = vec4<f32>(0.0788092, 0.36480793, 0.7264151, 0.0);
const GROUND_COLOR: vec4<f32> = vec4<f32>(0.35, 0.3, 0.35, 0.0);
const SUN_INTENSITY: f32 = 0.1;
const SUN_FOCUS: f32 = 500.0;
const epsilon: f32 = 1e-5;

@compute
@workgroup_size(8,8)
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

fn rand(seed: ptr<function,u32>) -> f32 {
    return f32(next_random_number(seed)) / 4294967295.0; // 2^32 - 1
}

fn rand_unit_sphere(seed: ptr<function, u32>) -> vec3<f32> {
    let x = rand_normal_dist(seed);
    let y = rand_normal_dist(seed);
    let z = rand_normal_dist(seed);

    return normalize(vec3(x, y, z));
}

fn rand_hemisphere(normal: vec3<f32>, seed: ptr<function, u32>) -> vec3<f32> {
    let dir = rand_unit_sphere(seed);
    return dir * sign(dot(normal, dir));
}

fn rand_normal_dist(seed: ptr<function, u32>) -> f32 {
    let theta = 2.0 * 3.1415926 * rand(seed);
    let rho = sqrt(-2.0 * log(rand(seed)));
    return rho * cos(theta);
}

fn rand_direction(seed: ptr<function,u32>) -> vec3<f32> {
    let x = rand_normal_dist(seed);
    let y = rand_normal_dist(seed);
    let z = rand_normal_dist(seed);

    return normalize(vec3(x, y, z));
}

fn next_random_number(seed: ptr<function,u32>) -> u32 {
    *seed = *seed * 747796405u + 2891336453u;
    var result: u32 = ((*seed >> ((*seed >> 28u) + 4u)) ^ *seed) * 277803737u;
    result = (result >> 22u) ^ result;
    return result;
}

fn rand_in_unit_disk(seed: ptr<function, u32>) -> vec2<f32> {
    let angle = rand(seed) * 2.0 * 3.1415926;
    let point_on_circle = vec2<f32>(cos(angle), sin(angle));
    return point_on_circle * sqrt(rand(seed));
}

fn reflectance(incident: vec3<f32>, normal: vec3<f32>, ior_a: f32, ior_b: f32) -> f32 {
    let cos_theta = dot(-incident, normal);
    let r0 = pow((ior_a - ior_b) / (ior_a + ior_b), 2.0);
    return r0 + (1.0 - r0) * pow(1.0 - cos_theta, 5.0);
}
// fn reflectance(uv: vec3<f32>, normal: vec3<f32>, ior_a: f32, ior_b: f32) -> f32 {
//     let refract_ratio = ior_a / ior_b;
//     let cos_angle_in = -dot(uv, normal);
//     let sin_sqr_angle_or = refract_ratio * refract_ratio * (1.0 - cos_angle_in * cos_angle_in);
//     if sin_sqr_angle_or >= 1.0 {
//         return 1.0;
//     }

//     let cos_angle_or = sqrt(1.0 - sin_sqr_angle_or);
//     let denominator_perp = ior_a * cos_angle_in + ior_b * cos_angle_or;
//     let denominator_parallel = ior_b * cos_angle_in + ior_a * cos_angle_or;
//     if abs(denominator_perp) < 1e-8 || abs(denominator_parallel) < 1e-8 {
//         return 1.0;
//     }

//     var r_perp = (ior_a * cos_angle_in - ior_b * cos_angle_or) / denominator_perp;
//     r_perp *= r_perp;
//     var r_parallel = (ior_b * cos_angle_in - ior_a * cos_angle_or) / denominator_parallel;
//     r_parallel *= r_parallel;
//     return (r_perp + r_parallel) * 0.5;
// }

// fn refract(uv: vec3<f32>, normal: vec3<f32>, ior_a: f32, ior_b: f32) -> vec3<f32> {
//     let refraction_ratio = ior_a / ior_b;
//     let cos_theta = min(dot(-uv, normal), 1.0);
//     let r_out_perp = refraction_ratio * (uv + cos_theta * normal);
//     let r_out_parallel = -sqrt(abs(1.0 - dot(r_out_perp, r_out_perp))) * normal;
//     return r_out_perp + r_out_parallel;
// }

fn refract(incident: vec3<f32>, normal: vec3<f32>, ior_in: f32, ior_out: f32) -> vec3<f32> {
    let eta = ior_in / ior_out;
    let cosi = clamp(dot(-incident, normal), -1.0, 1.0);
    let n = normal;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);

    if k < 0.0 {
        // Total internal reflection
        return reflect(incident, normal);
    } else {
        // Correct perpendicular + parallel decomposition
        return normalize(eta * incident + (eta * cosi - sqrt(k)) * n);
    }
}


// fn refract(uv: vec3<f32>, normal: vec3<f32>, ior_a: f32, ior_b: f32) -> vec3<f32> {
//     let refract_ratio = ior_a / ior_b;
//     let cos_angle_in = -dot(uv, normal);
//     let sin_sqr_angle_or = refract_ratio * refract_ratio * (1.0 - cos_angle_in * cos_angle_in);
//     if sin_sqr_angle_or > 1.0 {
//         return vec3<f32>(0.0);
//     }
//     return refract_ratio * uv + (refract_ratio * cos_angle_in - sqrt(1.0 - sin_sqr_angle_or)) * normal;
// }

fn get_environment_light(ray: Ray) -> vec4<f32> {
    let sky_gradient_t = pow(smoothstep(0.0, 0.4, ray.dir.y), 0.35);
    let ground_to_sky_t = smoothstep(-0.01, 0.0, ray.dir.y);
    let sky_gradient = mix(SKY_HORIZON, SKY_ZENITH, sky_gradient_t);
    let sun = pow(max(0.0, dot(ray.dir, vec3<f32>(0.1, 1.0, 0.1))), SUN_FOCUS) * SUN_INTENSITY;
    let composite = mix(GROUND_COLOR, sky_gradient, ground_to_sky_t) + sun * f32(ground_to_sky_t >= 1.0);
    return composite;
}

fn ray_sphere(ray: Ray, centre: vec3<f32>, radius: f32, cull_backface: bool) -> Hit {
    var hit: Hit;
    hit.dst = INF;

    let offset_ray_origin = ray.origin - centre;

    let a = dot(ray.dir, ray.dir);
    let b = 2.0 * dot(offset_ray_origin, ray.dir);
    let c = dot(offset_ray_origin, offset_ray_origin) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant >= 0.0 {
        let s = sqrt(discriminant);

        let dst_near = max(0.0, (-b - s) / (2.0 * a));
        let dst_far = (-b + s) / (2.0 * a);

        if dst_far >= 0.0 {
            let is_inside = dst_near == 0.0;
            if is_inside {
                hit.hit = true;
                hit.dst = dst_far;
            } else {
                hit.hit = true;
                hit.dst = dst_near;
            }
            hit.hit_point = ray.origin + ray.dir * hit.dst;
            if is_inside {
                hit.normal = -normalize(hit.hit_point - centre);
            } else {
                hit.normal = normalize(hit.hit_point - centre);
            }
            hit.backface = is_inside;
        }
    }

    return hit;
}

fn ray_triangle(ray: Ray, tri: Triangle, cull_backface: bool) -> Hit {
    var hit: Hit;
    hit.hit = false;
    let edge_ab = tri.v2 - tri.v1;
    let edge_ac = tri.v3 - tri.v1;
    let normal = cross(edge_ab, edge_ac);
    let ao = ray.origin - tri.v1;
    let dao = cross(ao, ray.dir);
    let determinant = -dot(ray.dir, normal);

    var keep: bool;
    if cull_backface {
        keep = determinant >= 1e-8;
    } else {
        keep = abs(determinant) >= 1e-8;
    }

    if !keep {
        return hit;
    }
    let inverse_determinant = 1.0 / determinant;

    let dst = dot(ao, normal) * inverse_determinant;
    let u = dot(edge_ac, dao) * inverse_determinant;
    let v = -dot(edge_ab, dao) * inverse_determinant;
    let w = 1.0 - u - v;

    if dst > epsilon && u >= 0.0 && v >= 0.0 && w >= 0.0 {
        hit.hit = true;
        hit.normal = normalize(tri.n1 * w + tri.n2 * u + tri.n3 * v) * sign(determinant);
        hit.backface = determinant < 0.0;
        hit.hit_point = ray.origin + ray.dir * dst;
        hit.dst = dst;
    }

    return hit;
}

fn ray_BVH(ray: Ray, ray_length: f32, node_offset: u32, tri_offset: u32, cull_backface: bool, stats: ptr<function, vec2<i32>>) -> Hit {
    var closest_hit: Hit;
    closest_hit.hit = false;
    closest_hit.dst = ray_length;

    var stack: array<u32,32>;
    var stack_index: u32 = 0u;
    stack[stack_index] = node_offset + 0u;
    stack_index += 1u;

    while stack_index > 0u {
        stack_index -= 1u;
        let node = nodes[stack[stack_index]];
        // Is Leaf node?
        if node.count > 0u {
            (*stats)[1] += i32(node.count); // Track triangle checks
            for (var j: u32 = 0u; j < node.count; j += 1u) {
                let tri = triangles[tri_offset + node.first + j];
                let hit = ray_triangle(ray, tri, cull_backface);
                if hit.hit && hit.dst < closest_hit.dst {
                    closest_hit = hit;
                }
            }
        } else { // Otherwise its root node, push children onto the stack
            let child_index_a = node_offset + node.left;
            let child_index_b = node_offset + node.right;
            let child_a = nodes[child_index_a];
            let child_b = nodes[child_index_b];
            let dst_a = ray_aabb_dist(ray, child_a.aabb_min, child_a.aabb_max, closest_hit.dst);
            let dst_b = ray_aabb_dist(ray, child_b.aabb_min, child_b.aabb_max, closest_hit.dst);
            (*stats)[0] += 2; // Track bounding box checks
            // Use index math to simplify code here:
            let left_is_closer = dst_a < dst_b;
            let near_dst = select(dst_b, dst_a, left_is_closer);
            let far_dst = select(dst_b, dst_a, !left_is_closer);
            let near_idx = select(child_index_b, child_index_a, left_is_closer);
            let far_idx = select(child_index_b, child_index_a, !left_is_closer);
            // Push farthest child first, (last on first off, last child gets checked first)
            if far_dst < closest_hit.dst { stack[stack_index] = far_idx; stack_index += 1u; }
            if near_dst < closest_hit.dst { stack[stack_index] = near_idx; stack_index += 1u; }
        }
    }
    return closest_hit;
}

fn ray_aabb_dist(ray: Ray, b_min: vec3<f32>, b_max: vec3<f32>, t: f32) -> f32 {
    let t1 = (b_min - ray.origin) * ray.inv_dir;
    let t2 = (b_max - ray.origin) * ray.inv_dir;
    var tmin = min(t1, t2);
    var tmax = max(t1, t2);

    let t_near = max(max(tmin.x, tmin.y), tmin.z);
    let t_far = min(min(tmax.x, tmax.y), tmax.z);

    let did_hit = t_far >= t_near && t_near < t && t_far > 0.0;
    if did_hit {
        return t_near;
    } else {
        return INF;
    }
}

fn calculate_ray_collions(ray: Ray, stats: ptr<function, vec2<i32>>) -> Hit {
    var closest_hit: Hit;
    closest_hit.hit = false;
    closest_hit.dst = INF;
    for (var i: u32 = 0u; i < scene.spheres; i += 1u) {
        var cull_backface = spheres[i].material.flag != MATERIAL_GLASS;
        let hit: Hit = ray_sphere(ray, spheres[i].position, spheres[i].radius, cull_backface);
        if hit.hit && hit.dst < closest_hit.dst {
            closest_hit = hit;
            closest_hit.material = spheres[i].material;
        }
    }
    var local_ray: Ray;
    local_ray.transmittance = vec4<f32>(0.0);
    local_ray.bounces = 0u;

    for (var i: u32 = 0u; i < scene.meshes; i += 1u) {
        let mesh = meshes[i];
        local_ray.origin = (mesh.world_to_model * vec4<f32>(ray.origin, 1.0)).xyz;
        local_ray.dir = normalize((mesh.world_to_model * vec4<f32>(ray.dir, 0.0)).xyz);
        local_ray.inv_dir = 1.0 / local_ray.dir;
        // Transform using matrices here instead of cpu, do later...
        var cull_backface = mesh.material.flag != MATERIAL_GLASS;

        let hit: Hit = ray_BVH(local_ray, INF, mesh.node_offset, mesh.triangle_offset, cull_backface, stats);
        if hit.hit {
            let local_hit_point = local_ray.origin + local_ray.dir * hit.dst;
            let world_hit_point = (mesh.model_to_world * vec4<f32>(local_hit_point, 1.0)).xyz;
            let world_dst = distance(ray.origin, world_hit_point);

            if world_dst < closest_hit.dst {
                closest_hit.hit = true;
                closest_hit.backface = hit.backface;
                closest_hit.normal = normalize((mesh.model_to_world * vec4<f32>(hit.normal, 0.0)).xyz);
                closest_hit.hit_point = world_hit_point;
                closest_hit.dst = world_dst;
                closest_hit.material = mesh.material;
            }
        }
    }

    return closest_hit;
}

fn trace(incident_ray: Ray, seed: ptr<function, u32>) -> vec4<f32> {
    var ray: Ray = incident_ray;
    ray.dir = normalize(ray.dir);
    ray.transmittance = vec4<f32>(1.0);
    var incoming_light = vec4<f32>(0.0);
    var _stats = vec2<i32>(0, 0);
    for (var i = i32(ray.bounces); i <= params.number_of_bounces; i += 1) {
        var hit = calculate_ray_collions(ray, &_stats);
        if !hit.hit {
            // Use get_environment_light if skybox is enabled
            if params.skybox != 0 {
                incoming_light += ray.transmittance * get_environment_light(ray);
            }
            break;
        }
        ray.origin = hit.hit_point;
        if hit.material.flag == MATERIAL_GLASS {
            if hit.backface {
                let x = ray.transmittance.rgb * exp(-hit.dst * hit.material.absorption.rgb * hit.material.absorption_strength);
                ray.transmittance = vec4(x.r, x.g, x.b, 1.0);
            }

            let ior_current = select(1.0, hit.material.ior, !hit.backface);
            let ior_next = select(hit.material.ior, 1.0, !hit.backface);

            var reflect_dir = reflect(ray.dir, hit.normal);
            var refract_dir = refract(ray.dir, hit.normal, ior_current, ior_next);

            // let reflect_weight = reflectance(ray.dir, hit.normal, ior_current, ior_next);
            // let refract_weight = 1.0 - reflect_weight;

            // let diffuse_dir = normalize(hit.normal + rand_direction(seed));

            // reflect_dir = normalize(mix(diffuse_dir, reflect_dir, hit.material.specular));
            // refract_dir = normalize(mix(-diffuse_dir, refract_dir, hit.material.smoothness));

            // let follow_reflection = rand(seed) <= reflect_weight;
            // ray.dir = select(refract_dir, reflect_dir, follow_reflection);
            ray.dir = normalize(refract_dir);
            let eps = max(1e-5, 1e-3 * hit.dst);
            ray.origin = hit.hit_point + eps * hit.normal * sign(dot(hit.normal, ray.dir));
        } else {
            let is_specular_bounce = hit.material.specular >= rand(seed);
            let diffuse_dir = rand_hemisphere(hit.normal, seed);
            let specular_dir = reflect(ray.dir, hit.normal);
            let emitted_light = hit.material.emission_color * hit.material.emission_strength;
            ray.dir = normalize(mix(diffuse_dir, specular_dir, hit.material.smoothness * f32(is_specular_bounce)));
            incoming_light += emitted_light * ray.transmittance;
            if is_specular_bounce {
                ray.transmittance *= hit.material.specular_color;
            } else {
                ray.transmittance *= hit.material.color;
            }
        }

        let p = max(ray.transmittance.r, max(ray.transmittance.g, ray.transmittance.b));
        if rand(seed) >= p {
                break;
        }
        ray.transmittance *= 1.0 / p;
        ray.inv_dir = 1.0 / ray.dir;
    }

    return incoming_light;
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
        ray.dir = normalize(scene.camera.lower_left_corner + pos.x * scene.camera.horizontal + pos.y * scene.camera.vertical - ray.origin);
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
    ray.dir = normalize(scene.camera.lower_left_corner + pos.x * scene.camera.horizontal + pos.y * scene.camera.vertical - ray.origin);
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
