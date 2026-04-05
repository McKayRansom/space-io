#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Must match MoonCraterUniform in Rust (std140):
// colors[2]: 32 bytes (offset 0)
// light_origin: vec2 8 bytes (offset 32)
// pixels..time: 7×4=28 bytes (offset 40)
// _pad0.._pad2: 3×4=12 bytes (offset 68)
// Total: 80 bytes = 5×16 ✓
struct MoonCraterUniform {
    colors: array<vec4<f32>, 2>,
    light_origin: vec2<f32>,
    pixels: f32,
    rotation: f32,
    time_speed: f32,
    light_border: f32,
    size: f32,
    seed: f32,
    time: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(2) @binding(0) var<uniform> material: MoonCraterUniform;

fn rand(coord: vec2<f32>) -> f32 {
    let m = vec2<f32>(1.0, 1.0) * round(material.size);
    let tiled = coord - m * floor(coord / m);
    return fract(sin(dot(tiled, vec2<f32>(12.9898, 78.233))) * 15.5453 * material.seed);
}

// by Leukbaars — https://www.shadertoy.com/view/4tK3zR
// Crater variant: returns 0 inside the circle, 1 outside (opposite of cloud version)
fn circle_noise(uv_in: vec2<f32>) -> f32 {
    let uv_y = floor(uv_in.y);
    var uv = uv_in;
    uv.x += uv_y * 0.31;
    let f = fract(uv);
    let h = rand(vec2<f32>(floor(uv.x), floor(uv_y)));
    let m = length(f - 0.25 - h * 0.5);
    let r = h * 0.25;
    return smoothstep(r - 0.1 * r, r, m);
}

// Multiply two circle_noise samples so craters appear where both overlap (=0 inside rim)
fn crater(uv: vec2<f32>) -> f32 {
    var c = 1.0;
    for (var i = 0; i < 2; i++) {
        c *= circle_noise(uv * material.size + (f32(i + 1) + 10.0) + vec2<f32>(material.time * material.time_speed, 0.0));
    }
    return 1.0 - c;
}

fn spherify(uv: vec2<f32>) -> vec2<f32> {
    let centered = uv * 2.0 - 1.0;
    let z = sqrt(max(0.0, 1.0 - dot(centered, centered)));
    let sphere = centered / (z + 1.0);
    return sphere * 0.5 + 0.5;
}

fn crater_rotate(coord_in: vec2<f32>, angle: f32) -> vec2<f32> {
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

    let d_circle = distance(uv, vec2<f32>(0.5));
    let d_light = distance(uv, material.light_origin);
    var a = step(d_circle, 0.49999);

    uv = crater_rotate(uv, material.rotation);
    uv = spherify(uv);

    let c1 = crater(uv);
    // Offset c2 slightly toward light to simulate crater rim shadow
    let c2 = crater(uv + (material.light_origin - 0.5) * 0.03);

    var col = material.colors[0];

    // Alpha: only draw where craters exist (c1 >= 0.5)
    a *= step(0.5, c1);

    // Shadow inside crater, far from light
    if c2 < c1 - (0.5 - d_light) * 2.0 {
        col = material.colors[1];
    }
    // Dark side of moon
    if d_light > material.light_border {
        col = material.colors[1];
    }

    // Re-clip to circle
    a *= step(d_circle, 0.5);

    return vec4<f32>(col.rgb, a * col.a);
}
