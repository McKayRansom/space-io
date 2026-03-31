#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Must match CloudMaterialUniform in Rust (ShaderType layout, std140 rules):
// colors: array<vec4<f32>, 4>  = 64 bytes  (offset 0)
// light_origin: vec2<f32>      =  8 bytes  (offset 64)
// pixels..time                 = 10×4=40b  (offset 72)
// _pad0, _pad1: u32            =  8 bytes  (offset 112)
// Total: 128 bytes = 8×16 ✓
struct CloudMaterialUniform {
    colors: array<vec4<f32>, 4>,
    light_origin: vec2<f32>,
    pixels: f32,
    rotation: f32,
    cloud_cover: f32,
    time_speed: f32,
    stretch: f32,
    cloud_curve: f32,
    light_border_1: f32,
    light_border_2: f32,
    size: f32,
    seed: f32,
    octaves: i32,
    time: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(2) @binding(0) var<uniform> material: CloudMaterialUniform;

// Square tiling (clouds wrap on a 1×1 grid, unlike land which uses 2×1)
fn rand(coord: vec2<f32>) -> f32 {
    let m = vec2<f32>(1.0, 1.0) * round(material.size);
    let tiled = coord - m * floor(coord / m);
    return fract(sin(dot(tiled, vec2<f32>(12.9898, 78.233))) * 15.5453 * material.seed);
}

fn noise(coord: vec2<f32>) -> f32 {
    let i = floor(coord);
    let f = fract(coord);
    let a = rand(i);
    let b = rand(i + vec2<f32>(1.0, 0.0));
    let c = rand(i + vec2<f32>(0.0, 1.0));
    let d = rand(i + vec2<f32>(1.0, 1.0));
    let cubic = f * f * (3.0 - 2.0 * f);
    return mix(a, b, cubic.x) + (c - a) * cubic.y * (1.0 - cubic.x) + (d - b) * cubic.x * cubic.y;
}

fn fbm(coord_in: vec2<f32>) -> f32 {
    var value = 0.0;
    var scale = 0.5;
    var coord = coord_in;
    for (var i = 0; i < 20; i++) {
        if i >= material.octaves { break; }
        value += noise(coord) * scale;
        coord *= 2.0;
        scale *= 0.5;
    }
    return value;
}

// by Leukbaars — https://www.shadertoy.com/view/4tK3zR
fn circle_noise(uv_in: vec2<f32>) -> f32 {
    let uv_y = floor(uv_in.y);
    var uv = uv_in;
    uv.x += uv_y * 0.31;
    let f = fract(uv);
    let h = rand(vec2<f32>(floor(uv.x), floor(uv_y)));
    let m = length(f - 0.25 - h * 0.5);
    let r = h * 0.25;
    return smoothstep(0.0, r, m * 0.75);
}

fn cloud_alpha(uv: vec2<f32>) -> f32 {
    var c_noise = 0.0;
    for (var i = 0; i < 9; i++) {
        let offset = vec2<f32>(material.time * material.time_speed, 0.0);
        c_noise += circle_noise(uv * material.size * 0.3 + (f32(i + 1) + 10.0) + offset);
    }
    return fbm(uv * material.size + c_noise + vec2<f32>(material.time * material.time_speed, 0.0));
}

fn spherify(uv: vec2<f32>) -> vec2<f32> {
    let centered = uv * 2.0 - 1.0;
    let z = sqrt(max(0.0, 1.0 - dot(centered, centered)));
    let sphere = centered / (z + 1.0);
    return sphere * 0.5 + 0.5;
}

fn cloud_rotate(coord_in: vec2<f32>, angle: f32) -> vec2<f32> {
    var coord = coord_in - vec2<f32>(0.5);
    let c = cos(angle);
    let s = sin(angle);
    coord = mat2x2<f32>(vec2<f32>(c, s), vec2<f32>(-s, c)) * coord;
    return coord + vec2<f32>(0.5);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv_raw = in.uv;
    var uv = floor(uv_raw * material.pixels) / material.pixels;

    let d_light = distance(uv, material.light_origin);
    let a = step(length(uv - vec2<f32>(0.5)), 0.49999);
    let d_to_center = distance(uv, vec2<f32>(0.5));

    uv = cloud_rotate(uv, material.rotation);
    uv = spherify(uv);
    // Slightly warp uv.y to curve clouds at the poles
    uv.y += smoothstep(0.0, material.cloud_curve, abs(uv.x - 0.4));

    let c = cloud_alpha(uv * vec2<f32>(1.0, material.stretch));

    // Color: bright cloud → shadow, based on cloud density and light distance
    var col = material.colors[0];
    if c < material.cloud_cover + 0.03 {
        col = material.colors[1];
    }
    if d_light + c * 0.2 > material.light_border_1 {
        col = material.colors[2];
    }
    if d_light + c * 0.2 > material.light_border_2 {
        col = material.colors[3];
    }

    // Clip clouds to the planet circle and step at cloud_cover threshold
    let c_masked = c * step(d_to_center, 0.5);
    return vec4<f32>(col.rgb, step(material.cloud_cover, c_masked) * a * col.a);
}
