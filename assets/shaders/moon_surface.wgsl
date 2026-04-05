#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Must match MoonSurfaceUniform in Rust (std140):
// colors[3]: 48 bytes (offset 0)
// light_origin: vec2 8 bytes (offset 48)
// pixels..should_dither: 11×4=44 bytes (offset 56)
// _pad0.._pad2: 3×4=12 bytes (offset 100)
// Total: 112 bytes = 7×16 ✓
struct MoonSurfaceUniform {
    colors: array<vec4<f32>, 3>,
    light_origin: vec2<f32>,
    pixels: f32,
    rotation: f32,
    time_speed: f32,
    dither_size: f32,
    light_border_1: f32,
    light_border_2: f32,
    size: f32,
    seed: f32,
    octaves: i32,
    time: f32,
    should_dither: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(2) @binding(0) var<uniform> material: MoonSurfaceUniform;

// Square tiling
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

// GLSL mod semantics (always non-negative for positive y)
fn glsl_mod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn dither(uv1: vec2<f32>, uv2: vec2<f32>) -> bool {
    let step = 2.0 / material.pixels;
    return glsl_mod(uv1.x + uv2.y, step) <= 1.0 / material.pixels;
}

fn surface_rotate(coord_in: vec2<f32>, angle: f32) -> vec2<f32> {
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
    var d_light = distance(uv, material.light_origin);
    let a = step(d_circle, 0.49999);

    let dith = dither(uv, uv_raw);
    uv = surface_rotate(uv, material.rotation);

    // FBM-driven terminator: noise shifts the light boundary creating terrain
    let fbm1 = fbm(uv);
    d_light += fbm(uv * material.size + fbm1 + vec2<f32>(material.time * material.time_speed, 0.0)) * 0.3;

    let dither_border = (1.0 / material.pixels) * material.dither_size;

    var col = material.colors[0];
    if d_light > material.light_border_1 {
        col = material.colors[1];
        if d_light < material.light_border_1 + dither_border && (dith || material.should_dither == 0u) {
            col = material.colors[0];
        }
    }
    if d_light > material.light_border_2 {
        col = material.colors[2];
        if d_light < material.light_border_2 + dither_border && (dith || material.should_dither == 0u) {
            col = material.colors[1];
        }
    }

    return vec4<f32>(col.rgb, a * col.a);
}
